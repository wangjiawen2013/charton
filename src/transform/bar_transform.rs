//! Bar layout: aggregate the rows, then decide each bar's lane and span.
//!
//! `transform_bar_data` prepares the table the bar renderer draws. It:
//!
//! 1. aggregates `y` per `(x, colour)` cell (a sum by default);
//! 2. solves the side-by-side lanes — how many bars share a category and which
//!    lane each row occupies — into the `sub_idx` / `groups_count` columns;
//! 3. carries a secondary `y2` measure through, so a bar can *float* between
//!    `y` and `y2` (a candlestick body, a waterfall step) instead of growing
//!    from zero. With `y2` present, colour is an attribute, not a lane: there is
//!    no Cartesian product and no dodging.
//!
//! The lane *arithmetic* lives in [`crate::position::Position`]; this transform
//! only decides which lane each row belongs to.

use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, MISSING_CATEGORY};
use crate::encode::y::StackMode;
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;

impl<T: Mark> Chart<T> {
    /// Aggregates, lays out the lanes and carries the floating `y2` bound.
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
        // A secondary measure turns the bar into a *floating* bar spanning
        // `y → y2` (a candlestick body, a waterfall step) instead of growing
        // from the baseline.
        let y2_field = self.encoding.y2.as_ref().map(|e| e.field.clone());

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

        // Capture prototypes for categorical restoration. Only the type metadata
        // is copied, never the row data.
        let x_col_proto = if !is_pie {
            Some(self.data.column(&x_field)?.type_prototype())
        } else {
            None
        };
        let c_col_proto = if has_grouping_color {
            Some(self.data.column(color_field.unwrap())?.type_prototype())
        } else {
            None
        };

        // --- STEP 2: Aggregate Data ---
        let mut group_map: AHashMap<(String, Option<String>), Vec<usize>> = AHashMap::new();
        let row_count = self.data.height();

        for i in 0..row_count {
            // Positional x drops a missing row; colour keeps it as the reserved
            // grey "NA" level so no valid observation is lost.
            let x_val = if is_pie {
                "all".to_string()
            } else {
                match self.data.get(&x_field, i).to_string() {
                    Some(value) => value,
                    None => continue,
                }
            };
            let c_val = if has_grouping_color {
                Some(match color_field {
                    Some(cf) => self
                        .data
                        .get(cf, i)
                        .to_string()
                        .unwrap_or_else(|| MISSING_CATEGORY.to_string()),
                    None => MISSING_CATEGORY.to_string(),
                })
            } else {
                None
            };
            group_map.entry((x_val, c_val)).or_default().push(i);
        }

        let y_col = self.data.column(&y_field)?;
        let mut lookup: AHashMap<(String, Option<String>), f64> = group_map
            .iter()
            .map(|(key, indices)| (key.clone(), agg_op.aggregate_by_index(y_col, indices)))
            .collect();

        // Aggregate the secondary measure over the same cells, without the
        // stacking/normalisation that applies to the primary one.
        let y2_lookup = if let Some(field) = &y2_field {
            let col = self.data.column(field)?;
            Some(
                group_map
                    .iter()
                    .map(|(key, indices)| (key.clone(), agg_op.aggregate_by_index(col, indices)))
                    .collect::<AHashMap<_, _>>(),
            )
        } else {
            None
        };

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
            self.data
                .column(color_field.unwrap())?
                .labels_with_missing()
        } else {
            vec![]
        };

        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_y2 = Vec::new();
        let mut final_color = Vec::new();
        // Lane helpers. A normal grouped bar dodges inside its category; a
        // floating bar is alone, so it always reports one lane.
        let mut f_groups_count = Vec::new();
        let mut f_sub_idx = Vec::new();

        let floating = y2_field.is_some();
        // Read the secondary measure for a cell, when one is configured.
        let y2_of = |key: &(String, Option<String>)| match &y2_lookup {
            Some(map) => map.get(key).copied().unwrap_or(f64::NAN),
            None => f64::NAN,
        };

        if floating {
            // A floating bar reads `y → y2` from one row, so `color` is an
            // attribute rather than a lane: no Cartesian product, no dodging,
            // only the cells that actually exist.
            let colors: Vec<Option<String>> = if has_grouping_color {
                c_uniques.iter().cloned().map(Some).collect()
            } else {
                vec![None]
            };
            for x in &x_uniques {
                for c in &colors {
                    let key = (x.clone(), c.clone());
                    if let Some(val) = lookup.get(&key).copied() {
                        final_x.push(x.clone());
                        final_y.push(val);
                        final_y2.push(y2_of(&key));
                        if has_grouping_color {
                            final_color.push(c.clone().unwrap_or_default());
                        }
                        f_groups_count.push(1.0);
                        f_sub_idx.push(0.0);
                    }
                }
            }
        } else if has_grouping_color {
            for x in &x_uniques {
                for (j, c) in c_uniques.iter().enumerate() {
                    let key = (x.clone(), Some(c.clone()));
                    let val = lookup.get(&key).cloned().unwrap_or(0.0);
                    final_x.push(x.clone());
                    final_color.push(c.clone());
                    final_y.push(val);
                    final_y2.push(y2_of(&key));
                    f_groups_count.push(c_uniques.len() as f64);
                    f_sub_idx.push(j as f64);
                }
            }
        } else {
            for x in &x_uniques {
                let key = (x.clone(), None);
                let val = lookup.get(&key).cloned().unwrap_or(0.0);
                final_x.push(x.clone());
                final_y.push(val);
                final_y2.push(y2_of(&key));
                f_groups_count.push(1.0);
                f_sub_idx.push(0.0);
            }
        }

        // --- STEP 5: Rebuild Dataset with Type Awareness ---
        let mut new_ds = Dataset::new();

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

        // A floating bar keeps its secondary bound so the renderer can span it.
        if let Some(field) = &y2_field {
            new_ds.add_column(
                field,
                ColumnVector::Float64 {
                    data: final_y2,
                    validity: None,
                },
            )?;
        }

        // 4. Layout Helpers (consistent with new Float64 variant)
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
