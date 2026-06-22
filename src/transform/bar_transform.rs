use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::encode::y::StackMode;
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;

impl<T: Mark> Chart<T> {
    pub(crate) fn transform_bar_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Context Extraction ---
        let y_enc = self
            .encoding
            .y
            .as_mut()
            .ok_or_else(|| ChartonError::Encoding("Y encoding missing".into()))?;
        let agg_op = y_enc.aggregate;
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X encoding missing".into()))?;
        let color_enc_opt = self.encoding.color.as_ref();

        let mut x_field = x_enc.field.clone();
        let y_field = y_enc.field.clone();

        // Check for Pie mode (empty X field)
        let is_pie = x_field.is_empty();
        if is_pie {
            y_enc.stack = StackMode::Stacked;
            x_field = format!("{}_virtual_root__", TEMP_SUFFIX);
        }

        let color_field = color_enc_opt.map(|ce| &ce.field);
        let has_grouping_color = if let Some(cf) = color_field {
            cf != &x_field
        } else {
            false
        };

        // Capture prototypes for categorical restoration
        let x_col_proto = if !is_pie {
            Some(self.data.column(&x_field)?.clone())
        } else {
            None
        };
        let c_col_proto = if has_grouping_color {
            Some(self.data.column(color_field.unwrap())?.clone())
        } else {
            None
        };

        // --- STEP 2: Aggregate Data ---
        let mut group_map: AHashMap<(String, Option<String>), Vec<usize>> = AHashMap::new();
        let row_count = self.data.height();

        for i in 0..row_count {
            let x_val = if is_pie {
                "all".to_string()
            } else {
                self.data
                    .get(&x_field, i)
                    .to_string()
                    .unwrap_or_else(|| "null".to_string())
            };
            let c_val = if has_grouping_color {
                color_field.map(|cf| {
                    self.data
                        .get(cf, i)
                        .to_string()
                        .unwrap_or_else(|| "null".to_string())
                })
            } else {
                None
            };
            group_map.entry((x_val, c_val)).or_default().push(i);
        }

        let y_col = self.data.column(&y_field)?;
        let mut lookup: AHashMap<(String, Option<String>), f64> = group_map
            .into_iter()
            .map(|(key, indices)| (key, agg_op.aggregate_by_index(y_col, &indices)))
            .collect();

        // --- STEP 3: Normalization ---
        if y_enc.normalize || y_enc.stack == StackMode::Normalize {
            let mut x_sums: AHashMap<String, f64> = AHashMap::new();
            for ((x, _), val) in &lookup {
                *x_sums.entry(x.clone()).or_insert(0.0) += val;
            }
            for ((x, _), val) in lookup.iter_mut() {
                let sum = x_sums.get(x).cloned().unwrap_or(0.0);
                *val = if sum != 0.0 { *val / sum } else { 0.0 };
            }
        }

        // --- STEP 4: Cartesian Product & Gap Filling ---
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
                    let val = lookup
                        .get(&(x.clone(), Some(c.clone())))
                        .cloned()
                        .unwrap_or(0.0);
                    final_x.push(x.clone());
                    final_color.push(c.clone());
                    final_y.push(val);
                }
            } else {
                let val = lookup.get(&(x.clone(), None)).cloned().unwrap_or(0.0);
                final_x.push(x.clone());
                final_y.push(val);
            }
        }

        // --- STEP 5: Rebuild Dataset with Type Awareness ---
        let mut new_ds = Dataset::new();
        let total_c = if has_grouping_color {
            c_uniques.len()
        } else {
            1
        };
        let total_rows = final_x.len();

        // 1. Restore X Axis (Categorical support)
        if is_pie {
            new_ds.add_column(
                "",
                ColumnVector::String {
                    data: final_x,
                    validity: None,
                },
            )?;
        } else {
            let x_cv = match x_col_proto {
                Some(ColumnVector::Categorical { values, .. }) => {
                    let val_map: AHashMap<&str, u32> = values
                        .iter()
                        .enumerate()
                        .map(|(idx, s)| (s.as_str(), idx as u32))
                        .collect();
                    let keys = final_x
                        .iter()
                        .map(|s| *val_map.get(s.as_str()).unwrap_or(&0))
                        .collect();
                    ColumnVector::Categorical {
                        keys,
                        values,
                        validity: None,
                    }
                }
                _ => ColumnVector::String {
                    data: final_x,
                    validity: None,
                },
            };
            new_ds.add_column(&x_field, x_cv)?;
        }

        // 2. Restore Color Axis (Categorical support)
        if has_grouping_color {
            let c_cv = match c_col_proto {
                Some(ColumnVector::Categorical { values, .. }) => {
                    let val_map: AHashMap<&str, u32> = values
                        .iter()
                        .enumerate()
                        .map(|(idx, s)| (s.as_str(), idx as u32))
                        .collect();
                    let keys = final_color
                        .iter()
                        .map(|s| *val_map.get(s.as_str()).unwrap_or(&0))
                        .collect();
                    ColumnVector::Categorical {
                        keys,
                        values,
                        validity: None,
                    }
                }
                _ => ColumnVector::String {
                    data: final_color,
                    validity: None,
                },
            };
            new_ds.add_column(color_field.unwrap(), c_cv)?;
        }

        // 3. Measures (Y is always F64 after aggregation)
        new_ds.add_column(
            &y_field,
            ColumnVector::Float64 {
                data: final_y,
                validity: None,
            },
        )?;

        // 4. Layout Helpers (consistent with new Float64 variant)
        let mut f_groups_count = Vec::with_capacity(total_rows);
        let mut f_sub_idx = Vec::with_capacity(total_rows);

        for _ in &x_uniques {
            for j in 0..total_c {
                f_groups_count.push(total_c as f64);
                f_sub_idx.push(j as f64);
            }
        }

        new_ds.add_column(
            format!("{}_groups_count", TEMP_SUFFIX),
            ColumnVector::Float64 {
                data: f_groups_count,
                validity: None,
            },
        )?;
        new_ds.add_column(
            format!("{}_sub_idx", TEMP_SUFFIX),
            ColumnVector::Float64 {
                data: f_sub_idx,
                validity: None,
            },
        )?;

        // --- STEP 6: Finalization ---
        self.data = new_ds;
        Ok(self)
    }
}
