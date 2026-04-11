use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, get_quantile};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

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
        // Preservation of appearance order prevents "flickering" of categories and dodge slots.
        let x_order = x_col.unique_values();

        let mut color_field_name: Option<String> = None;
        let mut color_to_idx = AHashMap::new();
        let mut color_order = Vec::new();

        if let Some(color_enc) = &self.encoding.color {
            let cf = color_enc.field.clone();
            color_order = self.data.column(&cf)?.unique_values();
            for (i, c) in color_order.iter().enumerate() {
                color_to_idx.insert(c.clone(), i as f64);
            }
            color_field_name = Some(cf);
        }

        let groups_count = if color_order.is_empty() {
            1.0
        } else {
            color_order.len() as f64
        };

        // --- STEP 3: Grouping phase ---
        // We use a HashMap for grouping indices, but we will iterate over x_order later to maintain sequence.
        let mut group_map: AHashMap<(String, Option<String>), Vec<usize>> = AHashMap::new();
        for i in 0..row_count {
            let x_val = x_col.get_str_or(i, "null");
            let c_val = color_field_name
                .as_ref()
                .map(|f| self.data.get_str_or(f, i, "null"));
            group_map.entry((x_val, c_val)).or_default().push(i);
        }

        // --- STEP 4: Parallel statistical computation ---
        // Instead of iterating over the HashMap directly (which is random),
        // we construct a flat list of tasks based on our stable X and Color orders.
        let mut tasks = Vec::new();
        for x_val in &x_order {
            if color_order.is_empty() {
                if let Some(indices) = group_map.get(&(x_val.clone(), None)) {
                    tasks.push((x_val.clone(), None, indices.clone()));
                }
            } else {
                for c_val in &color_order {
                    if let Some(indices) = group_map.get(&(x_val.clone(), Some(c_val.clone()))) {
                        tasks.push((x_val.clone(), Some(c_val.clone()), indices.clone()));
                    }
                }
            }
        }

        let stats_results: Vec<_> = tasks
            .maybe_into_par_iter()
            .map(|(x_val, c_val, indices)| {
                let mut vals: Vec<f64> = indices.iter().filter_map(|&i| y_col.get_f64(i)).collect();
                vals.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                if vals.is_empty() {
                    return (x_val, c_val, None);
                }

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

                let sub_idx = c_val
                    .as_ref()
                    .and_then(|c| color_to_idx.get(c))
                    .cloned()
                    .unwrap_or(0.0);

                (
                    x_val,
                    c_val,
                    Some(BoxStats {
                        q1,
                        median,
                        q3,
                        whisker_min,
                        whisker_max,
                        outliers,
                        sub_idx,
                    }),
                )
            })
            .collect();

        // --- STEP 5: Final Dataset Assembly ---
        let mut new_ds = Dataset::new();
        let result_len = stats_results.iter().filter(|(_, _, s)| s.is_some()).count();

        let mut final_x = Vec::with_capacity(result_len);
        let mut final_y = Vec::with_capacity(result_len);
        let mut final_c = Vec::with_capacity(result_len);
        let mut f_q1 = Vec::with_capacity(result_len);
        let mut f_median = Vec::with_capacity(result_len);
        let mut f_q3 = Vec::with_capacity(result_len);
        let mut f_min = Vec::with_capacity(result_len);
        let mut f_max = Vec::with_capacity(result_len);
        let mut f_sub_idx = Vec::with_capacity(result_len);
        let mut f_outliers = Vec::with_capacity(result_len);

        for (x, c, stats_opt) in stats_results {
            if let Some(s) = stats_opt {
                final_x.push(x);
                final_c.push(c.unwrap_or_else(|| "default".to_string()));
                final_y.push(s.median);
                f_q1.push(s.q1);
                f_median.push(s.median);
                f_q3.push(s.q3);
                f_min.push(s.whisker_min);
                f_max.push(s.whisker_max);
                f_sub_idx.push(s.sub_idx);
                f_outliers.push(format!("{:?}", s.outliers));
            }
        }

        // Global bounds injection for axis scaling
        if !final_y.is_empty() {
            final_y[0] = global_min;
            if final_y.len() > 1 {
                final_y[1] = global_max;
            }
        }

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

        self.data = new_ds;
        Ok(self)
    }
}

struct BoxStats {
    q1: f64,
    median: f64,
    q3: f64,
    whisker_min: f64,
    whisker_max: f64,
    outliers: Vec<f64>,
    sub_idx: f64,
}
