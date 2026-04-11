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
    /// This transformation ensures "Visual Continuity" by:
    /// 1. **Alignment**: Ensuring every color series has a value at every X-coordinate (Imputation).
    /// 2. **Ordering**: Sorting continuous/temporal scales while preserving discrete appearance order.
    /// 3. **Stacking**: Calculating y0 (baseline) and y1 (sum) based on the selected StackMode.
    pub(crate) fn transform_area_data(mut self) -> Result<Self, ChartonError> {
        // --- 1. Extract Encoding Metadata ---
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

        // Determine if X is a Quantitative/Temporal scale (requires sorting)
        // or a Categorical scale (requires order preservation).
        let x_col = self.data.column(&x_enc.field)?;
        let x_semantic = x_col.semantic_type();

        // Area/Line charts require sorting and numeric alignment if the data is
        // continuous (numbers) or temporal (dates), regardless of the final Scale choice.
        let is_continuous = matches!(
            x_semantic,
            SemanticType::Continuous | SemanticType::Temporal
        );

        let row_count = self.data.height();
        let x_col = self.data.column(x_field)?;
        let y_col = self.data.column(y_field)?;

        // --- 2. Build the Alignment Grid ---
        // We use a 2D-like lookup: Grid[X_Key][Color_Name] = Y_Value.
        // X_Key is a u64 (either string hash or f64 bits) for O(1) matching.
        let mut x_ticks_num: Vec<f64> = Vec::new();
        let mut x_ticks_str: Vec<String> = Vec::new();
        let mut x_set = AHashSet::new();
        let mut color_series: Vec<String> = Vec::new();
        let mut color_set = AHashSet::new();
        let mut grid: AHashMap<u64, AHashMap<String, f64>> = AHashMap::new();

        for i in 0..row_count {
            // Generate a stable u64 key for the X-axis coordinate
            let (x_key, x_val_f64, x_val_str) = if is_continuous {
                let v = x_col.get_f64(i).unwrap_or(0.0);
                (v.to_bits(), Some(v), None)
            } else {
                let s = x_col.get_str_or(i, "null");
                let mut hasher = ahash::AHasher::default();
                std::hash::Hash::hash(&s, &mut hasher);
                use std::hash::Hasher;
                (hasher.finish(), None, Some(s))
            };

            // Track unique X coordinates
            if x_set.insert(x_key) {
                if let Some(v) = x_val_f64 {
                    x_ticks_num.push(v);
                }
                if let Some(s) = x_val_str {
                    x_ticks_str.push(s);
                }
            }

            // Track unique Color series to ensure stable stacking order
            let c_val = color_field
                .map(|cf| self.data.get_str_or(cf, i, "default"))
                .unwrap_or_else(|| "default".to_string());

            if color_set.insert(c_val.clone()) {
                color_series.push(c_val.clone());
            }

            // Fill the grid with raw Y values
            let y_val = y_col.get_f64(i).unwrap_or(0.0);
            grid.entry(x_key).or_default().insert(c_val, y_val);
        }

        // Area charts on continuous scales must be sorted by X to prevent "zig-zag" artifacts
        if is_continuous {
            x_ticks_num
                .sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }

        if color_series.is_empty() {
            color_series.push("default".to_string());
        }

        // --- 3. Parallel Stacking & Imputation ---
        // We iterate through every X-tick. If a series is missing a value at a tick,
        // we "impute" it with 0.0 to ensure the area polygon remains continuous.
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

                // Re-derive the key for grid lookup
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

                // Offset for Streamgraphs (Center mode)
                let offset = if matches!(mode, StackMode::Center) {
                    -total / 2.0
                } else {
                    0.0
                };

                for c_name in &color_series {
                    // Imputation: unwrap_or(0.0) fills the gaps
                    let val = series_values.get(c_name).copied().unwrap_or(0.0);
                    let mut y0 = current_y;
                    let mut y1 = current_y + val;

                    // Apply relative stacking transformations
                    if matches!(mode, StackMode::Normalize) && total != 0.0 {
                        y0 /= total;
                        y1 /= total;
                    } else if matches!(mode, StackMode::Center) {
                        y0 += offset;
                        y1 += offset;
                    }

                    tick_data.push((out_f, out_s.clone(), c_name.clone(), y0, y1));
                    current_y += val;
                }
                tick_data
            })
            .collect();

        // --- 4. Final Dataset Construction ---
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
        // Clone for the max column, then move the original for the main Y field
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
