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
        // --- STEP 1: Context Extraction ---
        let y_enc = self.encoding.y.as_mut().unwrap();
        let agg_op = y_enc.aggregate;
        let x_enc = self.encoding.x.as_ref().unwrap();
        let color_enc_opt = self.encoding.color.as_ref();

        let mut x_field = x_enc.field.clone();
        let y_field = y_enc.field.clone();

        // Handle Pie/Single-axis mode: use a virtual root to group everything together
        let is_pie = x_field.is_empty();
        if is_pie {
            y_enc.stack = StackMode::Stacked;
            x_field = format!("{}_virtual_root__", TEMP_SUFFIX);
        }

        let color_field = color_enc_opt.map(|ce| &ce.field);
        let has_grouping_color = if let Some(ce) = color_enc_opt {
            &ce.field != &x_field
        } else {
            false
        };

        // --- STEP 2: Aggregate Data into a Lookup Map ---
        // We use AHashMap for O(1) lookups during the Cartesian product step
        let mut group_map: AHashMap<(String, Option<String>), Vec<usize>> = AHashMap::new();
        let row_count = self.data.height();

        for i in 0..row_count {
            let x_val = if is_pie {
                "all".to_string()
            } else {
                self.data.get_str_or(&x_field, i, "null")
            };
            let c_val = if has_grouping_color {
                color_field.map(|cf| self.data.get_str_or(cf, i, "null"))
            } else {
                None
            };
            group_map.entry((x_val, c_val)).or_default().push(i);
        }

        let y_col = self.data.column(&y_field)?;
        let mut lookup: AHashMap<(String, Option<String>), f64> = group_map
            .into_iter()
            .map(|(key, indices)| (key, agg_op.aggregate_by_index(&y_col, &indices)))
            .collect();

        // --- STEP 3: Normalization (if requested) ---
        if y_enc.normalize {
            let mut x_sums: AHashMap<String, f64> = AHashMap::new();
            for ((x, _), val) in &lookup {
                *x_sums.entry(x.clone()).or_insert(0.0) += val;
            }
            for ((x, _), val) in lookup.iter_mut() {
                let sum = x_sums.get(x).cloned().unwrap_or(0.0);
                *val = if sum != 0.0 { *val / sum } else { 0.0 };
            }
        }

        // --- STEP 4: Cartesian Product for Deterministic Order & Gap Filling ---
        // We use the stable unique_values() to define the "official" order of X and Color
        let x_uniques = if is_pie {
            vec!["all".to_string()]
        } else {
            self.data.column(&x_field)?.unique_values()
        };

        let c_uniques = if has_grouping_color {
            self.data.column(color_field.unwrap())?.unique_values()
        } else {
            vec![]
        };

        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_color = Vec::new();

        for x in &x_uniques {
            if has_grouping_color {
                for c in &c_uniques {
                    let val = lookup.remove(&(x.clone(), Some(c.clone()))).unwrap_or(0.0);
                    final_x.push(x.clone());
                    final_color.push(c.clone());
                    final_y.push(val);
                }
            } else {
                let val = lookup.remove(&(x.clone(), None)).unwrap_or(0.0);
                final_x.push(x.clone());
                final_y.push(val);
            }
        }

        // --- STEP 5: Rebuild Dataset ---
        let mut new_ds = Dataset::new();
        let x_col_name = if is_pie { "" } else { &x_field };

        new_ds.add_column(
            x_col_name,
            ColumnVector::String {
                data: final_x,
                validity: None,
            },
        )?;
        new_ds.add_column(&y_field, ColumnVector::F64 { data: final_y })?;

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
