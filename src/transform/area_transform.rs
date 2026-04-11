use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, SemanticType};
use crate::core::utils::IntoParallelizable;
use crate::encode::y::StackMode;
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::{AHashMap, AHashSet};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl<T: Mark> Chart<T> {
    /// Prepares data for area/bar charts based on the Scale type and StackMode.
    ///
    /// This version uses `unique_values()` to ensure that categorical axes (Discrete)
    /// maintain a stable order based on data appearance, preventing non-deterministic
    /// layout shifts caused by raw hash map iterations.
    pub(crate) fn transform_area_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encoding Metadata ---
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

        let x_col = self.data.column(x_field)?;
        let x_semantic = x_col.semantic_type();

        // Determine if X requires numeric sorting or order-preservation (categorical)
        let is_continuous = matches!(
            x_semantic,
            SemanticType::Continuous | SemanticType::Temporal
        );

        // --- STEP 2: Establish Deterministic Order for X and Color ---
        // We use unique_values() for Discrete scales to preserve appearance order.
        let x_ticks_str = if !is_continuous {
            x_col.unique_values()
        } else {
            Vec::new()
        };

        let color_series = if let Some(cf) = color_field {
            self.data.column(cf)?.unique_values()
        } else {
            vec!["default".to_string()]
        };

        // --- STEP 3: Build the Alignment Grid ---
        // Maps X-coordinates and Color-series into a lookup table for stacking.
        let mut x_ticks_num: Vec<f64> = Vec::new();
        let mut x_set = AHashSet::new();
        let mut grid: AHashMap<u64, AHashMap<String, f64>> = AHashMap::new();
        let row_count = self.data.height();
        let y_col = self.data.column(y_field)?;

        for i in 0..row_count {
            let (x_key, _x_val_f) = if is_continuous {
                let v = x_col.get_f64(i).unwrap_or(0.0);
                if x_set.insert(v.to_bits()) {
                    x_ticks_num.push(v);
                }
                (v.to_bits(), Some(v))
            } else {
                let s = x_col.get_str_or(i, "null");
                let mut hasher = ahash::AHasher::default();
                std::hash::Hash::hash(&s, &mut hasher);
                use std::hash::Hasher;
                (hasher.finish(), None)
            };

            let c_val = color_field
                .map(|cf| self.data.get_str_or(cf, i, "default"))
                .unwrap_or_else(|| "default".to_string());

            let y_val = y_col.get_f64(i).unwrap_or(0.0);
            grid.entry(x_key).or_default().insert(c_val, y_val);
        }

        // Continuous scales (Time/Linear) MUST be sorted by value to draw polygons correctly.
        if is_continuous {
            x_ticks_num
                .sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }

        // --- STEP 4: Parallel Stacking & Imputation ---
        // We iterate through our deterministic X and Color lists to calculate baselines.
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

                // Derive the X-key for grid lookup
                let (x_key, out_f, out_s) = if is_continuous {
                    let v = x_ticks_num[idx];
                    (v.to_bits(), Some(v), None)
                } else {
                    let s = &x_ticks_str[idx];
                    let mut hasher = ahash::AHasher::default();
                    std::hash::Hash::hash(s, &mut hasher);
                    use std::hash::Hasher;
                    (hasher.finish(), None, Some(s.clone()))
                };

                let series_values = grid.get(&x_key).unwrap();

                // Pre-calculate total height at this X-tick for Normalize/Center modes
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

                    // In Overlay mode (None), we skip missing values to avoid "dropping to zero"
                    // if a specific series doesn't exist at this X-tick.
                    if matches!(mode, StackMode::None) && maybe_val.is_none() {
                        continue;
                    }

                    // For stacking/normalizing, missing values MUST be 0.0 to keep series aligned.
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

        // --- STEP 5: Reconstruct Dataset ---
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
        if is_continuous {
            new_ds.add_column(x_field, ColumnVector::F64 { data: final_x_f })?;
        } else {
            new_ds.add_column(
                x_field,
                ColumnVector::String {
                    data: final_x_s,
                    validity: None,
                },
            )?;
        }

        let y0_name = format!("{}_{}_min", TEMP_SUFFIX, y_field);
        let y1_name = format!("{}_{}_max", TEMP_SUFFIX, y_field);

        new_ds.add_column(&y0_name, ColumnVector::F64 { data: final_y0 })?;
        new_ds.add_column(
            &y1_name,
            ColumnVector::F64 {
                data: final_y1.clone(),
            },
        )?;
        new_ds.add_column(y_field, ColumnVector::F64 { data: final_y1 })?;

        if let Some(cf) = color_field {
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
