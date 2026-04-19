use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl<T: Mark> Chart<T> {
    pub(crate) fn transform_errorbar_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encoding Context ---
        let x_field = &self.encoding.x.as_ref().unwrap().field;
        let y_field = &self.encoding.y.as_ref().unwrap().field;
        let color_enc_opt = self.encoding.color.as_ref();

        // Standardized temporary column names for the error boundaries
        let y_min_col = format!("{}_{}_min", TEMP_SUFFIX, y_field);
        let y_max_col = format!("{}_{}_max", TEMP_SUFFIX, y_field);

        // Determine if we are grouping by Color (Aesthetics)
        let group_by_color = if let Some(ce) = color_enc_opt {
            &ce.field != x_field
        } else {
            false
        };
        let color_field = color_enc_opt.map(|ce| &ce.field);

        // --- STEP 2: Unified Grouping ---
        // We use a HashMap to collect indices for each group.
        // Note: HashMaps are unordered, which we will fix in Step 4.
        let mut group_map: AHashMap<(String, Option<String>), Vec<usize>> = AHashMap::new();
        let x_col = self.data.column(x_field)?;
        let y_col = self.data.column(y_field)?;

        let row_count = self.data.height();
        for i in 0..row_count {
            let x_val = x_col.get_str_or(i, "null");
            let c_val = if group_by_color {
                color_field.map(|cf| self.data.get_str_or(cf, i, "null"))
            } else {
                None
            };
            group_map.entry((x_val, c_val)).or_default().push(i);
        }

        // --- STEP 3: Parallel Aggregation ---
        // Convert groups to a Vec for high-performance parallel processing.
        #[allow(clippy::type_complexity)]
        let groups: Vec<((String, Option<String>), Vec<usize>)> = group_map.into_iter().collect();

        #[allow(clippy::type_complexity)]
        let aggregated_results: Vec<((String, Option<String>), (f64, f64, f64))> = groups
            .maybe_into_par_iter()
            .map(|(key, indices)| {
                let mut sum = 0.0;
                let mut valid_count = 0;
                let mut vals = Vec::with_capacity(indices.len());

                for idx in indices {
                    if let Some(v) = y_col.get_f64(idx) {
                        sum += v;
                        vals.push(v);
                        valid_count += 1;
                    }
                }

                if valid_count == 0 {
                    return (key, (f64::NAN, f64::NAN, f64::NAN));
                }

                let mean = sum / valid_count as f64;

                // Calculate Sample Standard Deviation (ddof=1)
                let std = if valid_count > 1 {
                    let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                        / (valid_count - 1) as f64;
                    variance.sqrt()
                } else {
                    0.0 // Single point has no deviation
                };

                (key, (mean, mean - std, mean + std))
            })
            .collect();

        // --- STEP 4: Cartesian Product & Gap Filling (Deterministic Order) ---
        let lookup: AHashMap<(String, Option<String>), (f64, f64, f64)> =
            aggregated_results.into_iter().collect();

        let x_uniques = self.data.column(x_field)?.unique_values();
        let c_uniques = if group_by_color {
            self.data.column(color_field.unwrap())?.unique_values()
        } else {
            vec![]
        };

        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_ymin = Vec::new();
        let mut final_ymax = Vec::new();
        let mut final_color = Vec::new();

        // New helper columns for stable dodging
        let mut final_sub_idx = Vec::new();
        let mut final_groups_count = Vec::new();

        let total_groups = if group_by_color { c_uniques.len() } else { 1 } as f64;

        for x in &x_uniques {
            if group_by_color {
                for (c_idx, c) in c_uniques.iter().enumerate() {
                    let key = (x.clone(), Some(c.clone()));
                    let stats = lookup
                        .get(&key)
                        .cloned()
                        .unwrap_or((f64::NAN, f64::NAN, f64::NAN));

                    final_x.push(x.clone());
                    final_color.push(c.clone());
                    final_y.push(stats.0);
                    final_ymin.push(stats.1);
                    final_ymax.push(stats.2);

                    // Track the specific slot index and total slots for this X-category
                    final_sub_idx.push(c_idx as f64);
                    final_groups_count.push(total_groups);
                }
            } else {
                let key = (x.clone(), None);
                let stats = lookup
                    .get(&key)
                    .cloned()
                    .unwrap_or((f64::NAN, f64::NAN, f64::NAN));

                final_x.push(x.clone());
                final_y.push(stats.0);
                final_ymin.push(stats.1);
                final_ymax.push(stats.2);

                // Single group case: index 0 of 1
                final_sub_idx.push(0.0);
                final_groups_count.push(1.0);
            }
        }

        // --- STEP 5: Rebuild Dataset ---
        let mut new_ds = Dataset::new();

        // 1. Add primary axis (X)
        new_ds.add_column(
            x_field,
            ColumnVector::String {
                data: final_x,
                validity: None,
            },
        )?;

        // 2. Add statistical Y columns
        new_ds.add_column(y_field, ColumnVector::F64 { data: final_y })?;
        new_ds.add_column(&y_min_col, ColumnVector::F64 { data: final_ymin })?;
        new_ds.add_column(&y_max_col, ColumnVector::F64 { data: final_ymax })?;

        // 3. Add Color aesthetic column (if grouping is active)
        if group_by_color {
            new_ds.add_column(
                color_field.unwrap(),
                ColumnVector::String {
                    data: final_color,
                    validity: None,
                },
            )?;
        }

        // 4. Add Dodging helper columns (CRITICAL for alignment with Boxplots)
        new_ds.add_column(
            format!("{}_sub_idx", TEMP_SUFFIX),
            ColumnVector::F64 {
                data: final_sub_idx,
            },
        )?;
        new_ds.add_column(
            format!("{}_groups_count", TEMP_SUFFIX),
            ColumnVector::F64 {
                data: final_groups_count,
            },
        )?;

        // Finalize
        self.data = new_ds;
        Ok(self)
    }
}
