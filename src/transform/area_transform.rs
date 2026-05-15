use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::core::utils::IntoParallelizable;
use crate::encode::y::StackMode;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::scale::Scale;
use ahash::{AHashMap, AHashSet};
use std::hash::{Hash, Hasher};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl<T: Mark> Chart<T> {
    /// Prepares data for area charts by performing stacking and imputation.
    ///
    /// This implementation supports the latest physical ColumnVector types, ensuring
    /// temporal metadata (TimeUnit, Timezone) is preserved through the transformation.
    pub(crate) fn transform_area_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encoding & Scale Metadata ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X encoding missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y encoding missing".into()))?;

        let x_field = &x_enc.field;
        let y_field = &y_enc.field;
        let mode = &y_enc.stack;
        let color_field = self.encoding.color.as_ref().map(|c| &c.field);

        let x_scale_type = x_enc.scale_type.as_ref().ok_or_else(|| {
            ChartonError::Internal("Scale type must be resolved before transformation".into())
        })?;

        // Check if X is a continuous axis (including Temporal)
        let is_continuous = matches!(x_scale_type, Scale::Linear | Scale::Log | Scale::Temporal);

        // --- STEP 2: Establish Order & Capture Column Metadata ---
        let x_col = self.data.column(x_field)?;

        // We clone the column header/metadata to restore physical types in Step 5
        let x_prototype = x_col.clone();

        let x_ticks_str = if !is_continuous {
            x_col.unique_values()
        } else {
            Vec::new()
        };

        let color_series = if let Some(cf) = color_field {
            self.data.column(cf)?.unique_values()
        } else {
            vec![format!("{}_default", TEMP_SUFFIX)]
        };

        // --- STEP 3: Build the Alignment Grid ---
        let mut x_ticks_num: Vec<f64> = Vec::new();
        let mut x_set = AHashSet::new();
        let mut grid: AHashMap<u64, AHashMap<String, f64>> = AHashMap::new();
        let row_count = self.data.height();
        let y_col = self.data.column(y_field)?;

        for i in 0..row_count {
            let x_key = if is_continuous {
                // get_f64 automatically maps all numeric/temporal types to a double precision float
                let v = x_col.get_f64(i).unwrap_or(0.0);
                if x_set.insert(v.to_bits()) {
                    x_ticks_num.push(v);
                }
                v.to_bits()
            } else {
                let s = x_col.get_str_or(i, "null");
                let mut hasher = ahash::AHasher::default();
                s.hash(&mut hasher);
                hasher.finish()
            };

            let c_val = color_field
                .map(|cf| {
                    self.data
                        .get_str_or(cf, i, &format!("{}_default", TEMP_SUFFIX))
                })
                .unwrap_or_else(|| format!("{}_default", TEMP_SUFFIX));

            let y_val = y_col.get_f64(i).unwrap_or(0.0);
            grid.entry(x_key).or_default().insert(c_val, y_val);
        }

        if is_continuous {
            // Ensure stable rendering for polygons by sorting the continuous axis
            x_ticks_num.sort_unstable_by(|a, b| a.total_cmp(b));
        }

        // --- STEP 4: Stacking & Imputation ---
        let tick_count = if is_continuous {
            x_ticks_num.len()
        } else {
            x_ticks_str.len()
        };

        let stack_results: Vec<_> = (0..tick_count)
            .maybe_into_par_iter()
            .map(|idx| {
                let mut current_y = 0.0;
                let mut tick_data = Vec::with_capacity(color_series.len());

                let (x_key, out_f, out_s) = if is_continuous {
                    let v = x_ticks_num[idx];
                    (v.to_bits(), Some(v), None)
                } else {
                    let s = &x_ticks_str[idx];
                    let mut hasher = ahash::AHasher::default();
                    s.hash(&mut hasher);
                    (hasher.finish(), None, Some(s.clone()))
                };

                let series_values = grid.get(&x_key).unwrap();
                let total: f64 = color_series
                    .iter()
                    .map(|c| series_values.get(c).copied().unwrap_or(0.0))
                    .sum();
                let offset = if matches!(mode, StackMode::Center) {
                    -total / 2.0
                } else {
                    0.0
                };

                for c_name in &color_series {
                    let maybe_val = series_values.get(c_name).copied();

                    // In Overlay mode (None), skip missing series to prevent visual gaps
                    if matches!(mode, StackMode::None) && maybe_val.is_none() {
                        continue;
                    }

                    let val = maybe_val.unwrap_or(0.0);

                    let (y0, y1) = match mode {
                        StackMode::None => (val, val),
                        StackMode::Stacked => (current_y, current_y + val),
                        StackMode::Normalize => {
                            if total != 0.0 {
                                (current_y / total, (current_y + val) / total)
                            } else {
                                (0.0, 0.0)
                            }
                        }
                        StackMode::Center => (current_y + offset, current_y + val + offset),
                    };

                    tick_data.push((out_f, out_s.clone(), c_name.clone(), y0, y1));
                    if !matches!(mode, StackMode::None) {
                        current_y += val;
                    }
                }
                tick_data
            })
            .collect();

        // --- STEP 5: Reconstruction & Physical Type Restoration ---
        let mut final_x_f = Vec::new();
        let mut final_x_s = Vec::new();
        let mut final_y0 = Vec::new();
        let mut final_y1 = Vec::new();
        let mut final_c = Vec::new();

        for batch in stack_results {
            for (xf, xs, c, y0, y1) in batch {
                if let Some(v) = xf {
                    final_x_f.push(v);
                }
                if let Some(s) = xs {
                    final_x_s.push(s);
                }
                final_c.push(c);
                final_y0.push(y0);
                final_y1.push(y1);
            }
        }

        let mut new_ds = Dataset::new();

        // Restore X column based on the physical prototype
        if is_continuous {
            let restored_x = match x_prototype {
                ColumnVector::Datetime { unit, timezone, .. } => ColumnVector::Datetime {
                    // Apply round() to ensure values like 0.999... correctly snap back to
                    // integers, minimizing precision loss during float-to-int conversion.
                    data: final_x_f.into_iter().map(|v| v.round() as i64).collect(),
                    validity: None,
                    unit,
                    timezone,
                },
                ColumnVector::Date { .. } => ColumnVector::Date {
                    data: final_x_f.into_iter().map(|v| v.round() as i32).collect(),
                    validity: None,
                },
                ColumnVector::Duration { unit, .. } => ColumnVector::Duration {
                    data: final_x_f.into_iter().map(|v| v.round() as i64).collect(),
                    validity: None,
                    unit,
                },
                ColumnVector::Time { unit, .. } => ColumnVector::Time {
                    data: final_x_f.into_iter().map(|v| v.round() as i64).collect(),
                    validity: None,
                    unit,
                },
                // Fallback for all other numeric types to Float64 for coordinate precision
                _ => ColumnVector::Float64 {
                    data: final_x_f,
                    validity: None,
                },
            };
            new_ds.add_column(x_field, restored_x)?;
        } else {
            let restored_x = match x_prototype {
                // If original was Categorical, re-encode to preserve memory and speed
                ColumnVector::Categorical { values, .. } => {
                    let val_map: AHashMap<&str, u32> = values
                        .iter()
                        .enumerate()
                        .map(|(idx, s)| (s.as_str(), idx as u32))
                        .collect();

                    let keys: Vec<u32> = final_x_s
                        .iter()
                        .map(|s| *val_map.get(s.as_str()).unwrap_or(&0))
                        .collect();

                    ColumnVector::Categorical {
                        keys,
                        values,
                        validity: None,
                    }
                }
                // Fallback to standard String vector
                _ => ColumnVector::String {
                    data: final_x_s,
                    validity: None,
                },
            };
            new_ds.add_column(x_field, restored_x)?;
        }

        // --- STEP 6: Add Computed Y Bounds & Color ---
        // Y-axis results of stacking are always Float64 coordinates
        new_ds.add_column(
            &format!("{}_{}_min", TEMP_SUFFIX, y_field),
            ColumnVector::Float64 {
                data: final_y0,
                validity: None,
            },
        )?;
        new_ds.add_column(
            &format!("{}_{}_max", TEMP_SUFFIX, y_field),
            ColumnVector::Float64 {
                data: final_y1.clone(),
                validity: None,
            },
        )?;
        new_ds.add_column(
            y_field,
            ColumnVector::Float64 {
                data: final_y1,
                validity: None,
            },
        )?;

        if let Some(cf) = color_field {
            // Note: If color_field is also Categorical, you could apply the same re-encoding logic here
            new_ds.add_column(
                cf,
                ColumnVector::String {
                    data: final_c,
                    validity: None,
                },
            )?;
        }

        self.data = new_ds;
        Ok(self)
    }
}
