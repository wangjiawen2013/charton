use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, get_quantile};
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;

impl<T: Mark> Chart<T> {
    /// Performs high-performance statistical aggregation for Box Plots.
    ///
    /// This version ensures that gaps in categorical combinations are filled
    /// to maintain visual alignment (Dodge) and injects boundary points for scaling.
    pub(crate) fn transform_boxplot_data(mut self) -> Result<Self, ChartonError> {
        let x_name = &self.encoding.x.as_ref().unwrap().field;
        let y_name = &self.encoding.y.as_ref().unwrap().field;

        // --- STEP 1: Capture raw columns and calculate global Y-axis boundaries ---
        let x_col = self.data.column(x_name)?;
        let y_col = self.data.column(y_name)?;
        let row_count = self.data.height();

        // Prototype capture for type restoration
        let x_col_proto = x_col.clone();
        let mut color_col_proto: Option<ColumnVector> = None;

        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        for i in 0..row_count {
            if let Some(v) = y_col.get_f64(i) {
                if v < global_min {
                    global_min = v;
                }
                if v > global_max {
                    global_max = v;
                }
            }
        }

        // --- STEP 2: Establish Deterministic Order for X and Color ---
        let x_order = x_col.unique_values();
        let mut color_field_name: Option<String> = None;
        let mut color_order = Vec::new();

        if let Some(color_enc) = &self.encoding.color {
            let cf = color_enc.field.clone();
            let c_col = self.data.column(&cf)?;
            color_order = c_col.unique_values();
            color_col_proto = Some(c_col.clone());
            color_field_name = Some(cf);
        }

        let groups_count = if color_order.is_empty() {
            1.0
        } else {
            color_order.len() as f64
        };

        // --- STEP 3: Grouping phase ---
        let mut group_map: AHashMap<(String, Option<String>), Vec<usize>> = AHashMap::new();
        for i in 0..row_count {
            let x_val = x_col.get_str_or(i, "null");
            let c_val = color_field_name
                .as_ref()
                .map(|f| self.data.get_str_or(f, i, "null"));
            group_map.entry((x_val, c_val)).or_default().push(i);
        }

        // --- STEP 4: Cartesian Product & Statistical Computation ---
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_c = Vec::new();
        let mut f_q1 = Vec::new();
        let mut f_median = Vec::new();
        let mut f_q3 = Vec::new();
        let mut f_min = Vec::new();
        let mut f_max = Vec::new();
        let mut f_sub_idx = Vec::new();
        let mut f_outliers = Vec::new();

        for x_val in &x_order {
            let sub_tasks: Vec<(f64, Option<String>)> = if color_order.is_empty() {
                vec![(0.0, None)]
            } else {
                color_order
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (i as f64, Some(c.clone())))
                    .collect()
            };

            for (c_idx, c_val) in sub_tasks {
                final_x.push(x_val.clone());
                final_c.push(
                    c_val
                        .clone()
                        .unwrap_or_else(|| format!("{}_default", TEMP_SUFFIX)),
                );
                f_sub_idx.push(c_idx);

                if let Some(indices) = group_map.get(&(x_val.clone(), c_val)) {
                    let mut vals: Vec<f64> =
                        indices.iter().filter_map(|&i| y_col.get_f64(i)).collect();
                    vals.sort_unstable_by(|a, b| {
                        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                    });

                    if vals.is_empty() {
                        push_nan_row(
                            &mut final_y,
                            &mut f_q1,
                            &mut f_median,
                            &mut f_q3,
                            &mut f_min,
                            &mut f_max,
                            &mut f_outliers,
                        );
                    } else {
                        let q1 = get_quantile(&vals, 0.25);
                        let median = get_quantile(&vals, 0.50);
                        let q3 = get_quantile(&vals, 0.75);
                        let iqr = q3 - q1;
                        let lower_fence = q1 - 1.5 * iqr;
                        let upper_fence = q3 + 1.5 * iqr;

                        let mut outliers = Vec::new();
                        let mut whisker_min = q1;
                        let mut whisker_max = q3;

                        for &v in &vals {
                            if v < lower_fence || v > upper_fence {
                                outliers.push(v);
                            } else {
                                if v < whisker_min {
                                    whisker_min = v;
                                }
                                if v > whisker_max {
                                    whisker_max = v;
                                }
                            }
                        }

                        final_y.push(median);
                        f_q1.push(q1);
                        f_median.push(median);
                        f_q3.push(q3);
                        f_min.push(whisker_min);
                        f_max.push(whisker_max);
                        f_outliers.push(format!("{:?}", outliers));
                    }
                } else {
                    // Gap Filling: maintain alignment for missing data
                    push_nan_row(
                        &mut final_y,
                        &mut f_q1,
                        &mut f_median,
                        &mut f_q3,
                        &mut f_min,
                        &mut f_max,
                        &mut f_outliers,
                    );
                }
            }
        }

        // --- STEP 5: Boundary Injection (Invisible points for Y-Scale) ---
        if global_min.is_finite() {
            inject_boundary_row(
                global_min,
                &mut final_x,
                &mut final_y,
                &mut final_c,
                &mut f_q1,
                &mut f_median,
                &mut f_q3,
                &mut f_min,
                &mut f_max,
                &mut f_sub_idx,
                &mut f_outliers,
            );
            inject_boundary_row(
                global_max,
                &mut final_x,
                &mut final_y,
                &mut final_c,
                &mut f_q1,
                &mut f_median,
                &mut f_q3,
                &mut f_min,
                &mut f_max,
                &mut f_sub_idx,
                &mut f_outliers,
            );
        }

        // --- STEP 6: Final Dataset Assembly with Categorical Restoration ---
        let mut new_ds = Dataset::new();
        let result_len = final_x.len();

        // Restore X axis (Categorical vs String)
        let x_cv = match x_col_proto {
            ColumnVector::Categorical { values, .. } => {
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
        new_ds.add_column(x_name, x_cv)?;

        // Build Statistical Columns (Now using Float64)
        new_ds.add_column(
            y_name,
            ColumnVector::Float64 {
                data: final_y,
                validity: None,
            },
        )?;
        new_ds.add_column(
            format!("{}_q1", TEMP_SUFFIX),
            ColumnVector::Float64 {
                data: f_q1,
                validity: None,
            },
        )?;
        new_ds.add_column(
            format!("{}_median", TEMP_SUFFIX),
            ColumnVector::Float64 {
                data: f_median,
                validity: None,
            },
        )?;
        new_ds.add_column(
            format!("{}_q3", TEMP_SUFFIX),
            ColumnVector::Float64 {
                data: f_q3,
                validity: None,
            },
        )?;
        new_ds.add_column(
            format!("{}_min", TEMP_SUFFIX),
            ColumnVector::Float64 {
                data: f_min,
                validity: None,
            },
        )?;
        new_ds.add_column(
            format!("{}_max", TEMP_SUFFIX),
            ColumnVector::Float64 {
                data: f_max,
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
        new_ds.add_column(
            format!("{}_groups_count", TEMP_SUFFIX),
            ColumnVector::Float64 {
                data: vec![groups_count; result_len],
                validity: None,
            },
        )?;
        new_ds.add_column(
            format!("{}_outliers", TEMP_SUFFIX),
            ColumnVector::String {
                data: f_outliers,
                validity: None,
            },
        )?;

        // Restore Color axis
        if let Some(ref f) = color_field_name {
            let c_cv = match color_col_proto {
                Some(ColumnVector::Categorical { values, .. }) => {
                    let val_map: AHashMap<&str, u32> = values
                        .iter()
                        .enumerate()
                        .map(|(idx, s)| (s.as_str(), idx as u32))
                        .collect();
                    let keys = final_c
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
                    data: final_c,
                    validity: None,
                },
            };
            new_ds.add_column(f, c_cv)?;
        }

        self.data = new_ds;
        Ok(self)
    }
}

/// Helper to push NaN rows for gaps to maintain layout consistency.
fn push_nan_row(
    y: &mut Vec<f64>,
    q1: &mut Vec<f64>,
    med: &mut Vec<f64>,
    q3: &mut Vec<f64>,
    min: &mut Vec<f64>,
    max: &mut Vec<f64>,
    out: &mut Vec<String>,
) {
    y.push(f64::NAN);
    q1.push(f64::NAN);
    med.push(f64::NAN);
    q3.push(f64::NAN);
    min.push(f64::NAN);
    max.push(f64::NAN);
    out.push("[]".to_string());
}

/// Helper to inject boundary points that ensure the Y-axis scale covers outliers.
#[allow(clippy::too_many_arguments)]
fn inject_boundary_row(
    val: f64,
    x: &mut Vec<String>,
    y: &mut Vec<f64>,
    c: &mut Vec<String>,
    q1: &mut Vec<f64>,
    med: &mut Vec<f64>,
    q3: &mut Vec<f64>,
    min: &mut Vec<f64>,
    max: &mut Vec<f64>,
    s_idx: &mut Vec<f64>,
    out: &mut Vec<String>,
) {
    x.push(format!("{}_boundary", TEMP_SUFFIX));
    y.push(val);
    c.push(format!("{}_default", TEMP_SUFFIX));
    q1.push(f64::NAN);
    med.push(f64::NAN);
    q3.push(f64::NAN);
    min.push(f64::NAN);
    max.push(f64::NAN);
    s_idx.push(0.0);
    out.push("[]".to_string());
}
