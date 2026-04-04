use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;
use rayon::prelude::*;
use std::collections::HashSet;

impl<T: Mark> Chart<T> {
    pub(crate) fn transform_errorbar_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encoding Context ---
        let x_field = &self.encoding.x.as_ref().unwrap().field;
        let y_field = &self.encoding.y.as_ref().unwrap().field;
        let color_enc_opt = self.encoding.color.as_ref();

        let y_min_col = format!("{}_{}_min", TEMP_SUFFIX, y_field);
        let y_max_col = format!("{}_{}_max", TEMP_SUFFIX, y_field);

        // Determine if we are grouping by Color as well
        let group_by_color = if let Some(ce) = color_enc_opt {
            &ce.field != x_field
        } else {
            false
        };
        let color_field = color_enc_opt.map(|ce| &ce.field);

        // --- STEP 2: Unified Grouping ---
        // Key: (X_Value, Option<Color_Value>) -> Value: Vec<indices>
        let mut group_map: AHashMap<(String, Option<String>), Vec<usize>> = AHashMap::new();
        let x_col = self.data.column(x_field)?;
        let y_col = self.data.column(y_field)?;

        let row_count = self.data.height();
        for i in 0..row_count {
            let x_val = x_col.get_as_string(i).unwrap_or_else(|| "null".to_string());
            let c_val = if group_by_color {
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

        // --- STEP 3: Parallel Aggregation ---
        // Convert groups to a Vec for Rayon processing
        let groups: Vec<((String, Option<String>), Vec<usize>)> = group_map.into_iter().collect();

        let aggregated_results: Vec<((String, Option<String>), (f64, f64, f64))> = groups
            .into_par_iter()
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

                // Sample Standard Deviation (ddof=1)
                let std = if valid_count > 1 {
                    let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                        / (valid_count - 1) as f64;
                    variance.sqrt()
                } else {
                    f64::NAN
                };

                (key, (mean, mean - std, mean + std))
            })
            .collect();

        // --- STEP 4: Cartesian Product & Gap Filling ---
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_ymin = Vec::new();
        let mut final_ymax = Vec::new();
        let mut final_color = Vec::new();

        // Map aggregated data for quick lookup during gap filling
        let lookup: AHashMap<(String, Option<String>), (f64, f64, f64)> =
            aggregated_results.into_iter().collect();

        // Get unique X and Color values while preserving order (using HashSet + Vec for stability)
        let mut x_uniques = Vec::new();
        let mut x_seen = HashSet::new();
        let mut c_uniques = Vec::new();
        let mut c_seen = HashSet::new();

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
        x_uniques.sort(); // Optional: Ensure stable visual order
        c_uniques.sort();

        // Perform Cartesian Product
        for x in &x_uniques {
            if group_by_color {
                for c in &c_uniques {
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
            }
        }

        // --- STEP 5: Rebuild Dataset ---
        let mut new_ds = Dataset::new();
        new_ds.add_column(
            x_field,
            ColumnVector::String {
                data: final_x,
                validity: None,
            },
        )?;
        new_ds.add_column(y_field, ColumnVector::F64 { data: final_y })?;
        new_ds.add_column(&y_min_col, ColumnVector::F64 { data: final_ymin })?;
        new_ds.add_column(&y_max_col, ColumnVector::F64 { data: final_ymax })?;

        if group_by_color {
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
