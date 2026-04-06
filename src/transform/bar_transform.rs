use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::core::utils::{IntoParallelizable, Parallelizable};
use crate::encode::y::StackMode;
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::{AHashMap, AHashSet};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl<T: Mark> Chart<T> {
    pub(crate) fn transform_bar_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encoding Context ---
        let y_enc = self.encoding.y.as_mut().unwrap();
        let agg_op = y_enc.aggregate;
        let x_enc = self.encoding.x.as_ref().unwrap();
        let color_enc_opt = self.encoding.color.as_ref();

        let mut x_field = x_enc.field.clone();
        let y_field = &y_enc.field;

        // --- PIE/SINGLE-AXIS MODE HANDLING ---
        // If x_field is empty, it's a Pie chart. We use a virtual "root" group.
        let is_pie = x_field.is_empty();
        if is_pie {
            y_enc.stack = StackMode::Stacked;
            x_field = format!("{}_virtual_root__", TEMP_SUFFIX);
        }

        // Determine grouping strategy
        let color_field = color_enc_opt.map(|ce| &ce.field);
        let has_grouping_color = if let Some(ce) = color_enc_opt {
            &ce.field != &x_field
        } else {
            false
        };

        // --- STEP 2: Unified Grouping (Using ahash) ---
        let mut group_map: AHashMap<(String, Option<String>), Vec<usize>> = AHashMap::new();
        let row_count = self.data.height();

        for i in 0..row_count {
            let x_val = if is_pie {
                "all".to_string()
            } else {
                self.data
                    .column(&x_field)?
                    .get_as_string(i)
                    .unwrap_or_else(|| "null".to_string())
            };

            let c_val = if has_grouping_color {
                Some(
                    self.data
                        .column(color_field.unwrap())?
                        .get_as_string(i)
                        .unwrap_or_else(|| "null".to_string()),
                )
            } else {
                None
            };
            group_map.entry((x_val, c_val)).or_default().push(i);
        }

        // --- STEP 3: Parallel Aggregation (Using Rayon) ---
        let y_col = self.data.column(y_field)?;
        let groups: Vec<((String, Option<String>), Vec<usize>)> = group_map.into_iter().collect();

        let mut aggregated_results: Vec<((String, Option<String>), f64)> = groups
            .maybe_into_par_iter()
            .map(|(key, indices)| {
                // Use the AggregateOp enum to compute the value
                let val = agg_op.aggregate_by_index(&y_col, &indices);
                (key, val)
            })
            .collect();

        // --- STEP 4: Normalization (Optional) ---
        if y_enc.normalize {
            // Calculate sum per X-group
            let mut x_sums: AHashMap<String, f64> = AHashMap::new();
            for ((x, _), val) in &aggregated_results {
                *x_sums.entry(x.clone()).or_insert(0.0) += val;
            }

            // Normalize values in parallel
            (&mut aggregated_results)
                .maybe_par_iter()
                .for_each(|((x, _), val)| {
                    let sum = x_sums.get(x).cloned().unwrap_or(1.0);
                    *val = if sum != 0.0 { *val / sum } else { 0.0 };
                });
        }

        // --- STEP 5: Cartesian Product & Gap Filling ---
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_color = Vec::new();

        let lookup: AHashMap<(String, Option<String>), f64> =
            aggregated_results.into_iter().collect();

        // Extract unique keys for X and Color to build the grid
        let mut x_uniques = Vec::new();
        let mut c_uniques = Vec::new();
        let mut x_seen = AHashSet::new();
        let mut c_seen = AHashSet::new();

        for ((x, c), _) in &lookup {
            if x_seen.insert(x.clone()) {
                x_uniques.push(x.clone());
            }
            if let Some(color_val) = c {
                if c_seen.insert(color_val.clone()) {
                    c_uniques.push(color_val.clone());
                }
            }
        }

        // Build the expanded dataset
        for x in &x_uniques {
            if has_grouping_color {
                for c in &c_uniques {
                    let key = (x.clone(), Some(c.clone()));
                    let val = lookup.get(&key).cloned().unwrap_or(0.0); // Fill gaps with 0

                    final_x.push(x.clone());
                    final_color.push(c.clone());
                    final_y.push(val);
                }
            } else {
                let key = (x.clone(), None);
                let val = lookup.get(&key).cloned().unwrap_or(0.0);

                final_x.push(x.clone());
                final_y.push(val);
            }
        }

        // --- STEP 6: Rebuild Dataset ---
        let mut new_ds = Dataset::new();

        // Use the original x_field name (or "" for pie)
        let x_col_name = if is_pie { "" } else { &x_field };
        new_ds.add_column(
            x_col_name,
            ColumnVector::String {
                data: final_x,
                validity: None,
            },
        )?;
        new_ds.add_column(y_field, ColumnVector::F64 { data: final_y })?;

        if has_grouping_color {
            new_ds.add_column(
                color_field.unwrap(),
                ColumnVector::String {
                    data: final_color,
                    validity: None,
                },
            )?;
        }

        self.data = new_ds;
        Ok(self)
    }
}
