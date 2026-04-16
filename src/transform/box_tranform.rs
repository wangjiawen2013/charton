use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, get_quantile};
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;

impl<T: Mark> Chart<T> {
    /// Performs high-performance statistical aggregation for Box Plots.
    ///
    /// This version uses `unique_values()` to ensure that both X-axis categories
    /// and Color dodge slots maintain a stable, appearance-based order.
    pub(crate) fn transform_boxplot_data(mut self) -> Result<Self, ChartonError> {
        let x_name = &self.encoding.x.as_ref().unwrap().field;
        let y_name = &self.encoding.y.as_ref().unwrap().field;

        // --- STEP 1: Capture raw columns and calculate global Y-axis boundaries ---
        let x_col = self.data.column(x_name)?;
        let y_col = self.data.column(y_name)?;
        let row_count = self.data.height();

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
            color_order = self.data.column(&cf)?.unique_values();
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
        // We iterate through every possible X + Color combination to ensure
        // gaps are preserved (Gap Filling).
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
            // Handle both grouped and non-grouped scenarios
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
                    // Force a placeholder row for missing data to maintain DODGE alignment
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

        // --- STEP 5: Boundary Injection ---
        // Append two extra rows with global min/max.
        // This ensures the Y-axis scale covers all data including outliers across all groups.
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

        // --- STEP 6: Final Dataset Assembly ---
        let mut new_ds = Dataset::new();
        let result_len = final_x.len();

        new_ds.add_column(
            x_name,
            ColumnVector::String {
                data: final_x,
                validity: None,
            },
        )?;
        new_ds.add_column(y_name, ColumnVector::F64 { data: final_y })?;
        new_ds.add_column(
            &format!("{}_q1", TEMP_SUFFIX),
            ColumnVector::F64 { data: f_q1 },
        )?;
        new_ds.add_column(
            &format!("{}_median", TEMP_SUFFIX),
            ColumnVector::F64 { data: f_median },
        )?;
        new_ds.add_column(
            &format!("{}_q3", TEMP_SUFFIX),
            ColumnVector::F64 { data: f_q3 },
        )?;
        new_ds.add_column(
            &format!("{}_min", TEMP_SUFFIX),
            ColumnVector::F64 { data: f_min },
        )?;
        new_ds.add_column(
            &format!("{}_max", TEMP_SUFFIX),
            ColumnVector::F64 { data: f_max },
        )?;
        new_ds.add_column(
            &format!("{}_sub_idx", TEMP_SUFFIX),
            ColumnVector::F64 { data: f_sub_idx },
        )?;
        new_ds.add_column(
            &format!("{}_groups_count", TEMP_SUFFIX),
            ColumnVector::F64 {
                data: vec![groups_count; result_len],
            },
        )?;
        new_ds.add_column(
            &format!("{}_outliers", TEMP_SUFFIX),
            ColumnVector::String {
                data: f_outliers,
                validity: None,
            },
        )?;

        if let Some(ref f) = color_field_name {
            new_ds.add_column(
                f,
                ColumnVector::String {
                    data: final_c,
                    validity: None,
                },
            )?;
        }
        println!("{:?}", new_ds);
        self.data = new_ds;
        Ok(self)
    }
}

// Helper to push NaN rows for gaps
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

/// Helper to inject invisible boundary points to ensure the Y-axis scale
/// covers the absolute global range (including outliers).
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
    // STRATEGY: "Cloaking" (data exists, but is invisible)
    // We use a unique boundary name that the renderer will ignore.
    // This forced value in the 'y' column ensures the Scale accommodates
    // the furthest outliers without drawing any visual box marks.
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
