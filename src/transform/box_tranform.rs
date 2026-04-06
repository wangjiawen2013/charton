use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, get_quantile};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::{AHashMap, AHashSet};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl<T: Mark> Chart<T> {
    /// Performs high-performance statistical aggregation for Box Plots.
    ///
    /// This transform replaces the previous Polars-based logic with a native Rust implementation:
    /// 1. Groups data by X-axis and Color categories using a high-speed HashMap.
    /// 2. Calculates global Y-axis bounds to ensure consistent scaling for whiskers and outliers.
    /// 3. Assigns fixed "Dodge" indices to colors to maintain consistent visual slots even when data is missing.
    /// 4. Computes the 5-number summary (Min, Q1, Median, Q3, Max) and identifies outliers in parallel.
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

        // --- STEP 2: Grouping phase ---
        // Identifies unique combinations of X and Color to form boxplot groups.
        let mut color_field_name: Option<String> = None;
        if let Some(color_enc) = &self.encoding.color {
            color_field_name = Some(color_enc.field.clone());
        }

        let mut group_map: AHashMap<(String, Option<String>), Vec<usize>> = AHashMap::new();
        for i in 0..row_count {
            let x_val = x_col.get_as_string(i).unwrap_or_else(|| "null".to_string());
            let c_val = color_field_name.as_ref().map(|f| {
                self.data
                    .column(f)
                    .unwrap()
                    .get_as_string(i)
                    .unwrap_or_else(|| "null".to_string())
            });
            group_map.entry((x_val, c_val)).or_default().push(i);
        }

        // --- STEP 3: Fixed indexing for Dodge alignment ---
        // Ensures that each color occupies a consistent horizontal "slot" within each X category.
        let mut color_to_idx = AHashMap::new();
        if let Some(ref _f) = color_field_name {
            let mut unique_colors: Vec<String> = group_map
                .keys()
                .filter_map(|k| k.1.clone())
                .collect::<AHashSet<_>>()
                .into_iter()
                .collect();
            unique_colors.sort_unstable();
            for (i, c) in unique_colors.into_iter().enumerate() {
                color_to_idx.insert(c, i as f64);
            }
        }
        let groups_count = if color_to_idx.is_empty() {
            1.0
        } else {
            color_to_idx.len() as f64
        };

        // --- STEP 4: Parallel statistical computation ---
        // Sorts values and calculates boxplot metrics (quantiles, whiskers, outliers) for each group.
        let groups: Vec<((String, Option<String>), Vec<usize>)> = group_map.into_iter().collect();

        let stats_results: Vec<_> = groups
            .maybe_into_par_iter()
            .map(|((x_val, c_val), indices)| {
                // Extract and sort values using unstable sort for maximum performance
                let mut vals: Vec<f64> = indices.iter().filter_map(|&i| y_col.get_f64(i)).collect();
                vals.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                if vals.is_empty() {
                    return (x_val, c_val, None);
                }

                // Calculate 5-number summary using linear interpolation
                let q1 = get_quantile(&vals, 0.25);
                let median = get_quantile(&vals, 0.50);
                let q3 = get_quantile(&vals, 0.75);

                // Tukey's fences for whisker and outlier detection (1.5 * IQR)
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
        // Collects results into a new Columnar Dataset, injecting global bounds for axis scaling.
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
                final_y.push(s.median); // Using Median as the primary Y-axis anchor
                f_q1.push(s.q1);
                f_median.push(s.median);
                f_q3.push(s.q3);
                f_min.push(s.whisker_min);
                f_max.push(s.whisker_max);
                f_sub_idx.push(s.sub_idx);
                f_outliers.push(format!("{:?}", s.outliers));
            }
        }

        // Inject global min/max into the first rows to force correct Y-axis domain detection
        if !final_y.is_empty() {
            final_y[0] = global_min;
            if final_y.len() > 1 {
                final_y[1] = global_max;
            }
        }

        // Build the final Columnar representation
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

/// Internal statistical structure to hold box plot metrics for a single group.
struct BoxStats {
    q1: f64,
    median: f64,
    q3: f64,
    whisker_min: f64,
    whisker_max: f64,
    outliers: Vec<f64>,
    /// The relative position index of the color group used for horizontal "Dodge" alignment.
    /// This ensures each color has a fixed slot even if some categories are missing data.
    sub_idx: f64,
}
