use crate::chart::Chart;
use crate::scale::Scale;
use crate::error::ChartonError;
use crate::mark::Mark;
use polars::prelude::*;

impl<T: Mark> Chart<T> {
    // Handle grouping and aggregation of data for rect chart
    pub(crate) fn transform_rect_data(mut self) -> Result<Self, ChartonError> {
        // Get encodings - we know these exist based on earlier validation
        let x_encoding = self.encoding.x.as_ref().unwrap();
        let y_encoding = self.encoding.y.as_ref().unwrap();
        let color_encoding = self.encoding.color.as_ref().unwrap();

        // Determine if x and y values are discrete
        let x_scale_type = self.get_x_scale_type();
        let y_scale_type = self.get_y_scale_type();

        let x_is_discrete = matches!(x_scale_type.as_ref(), Some(Scale::Discrete));
        let y_is_discrete = matches!(y_scale_type.as_ref(), Some(Scale::Discrete));

        // Store bin information for later use
        let mut x_bin_labels: Option<Vec<String>> = None;
        let mut y_bin_labels: Option<Vec<String>> = None;
        let mut x_bin_middles: Option<Vec<f64>> = None;
        let mut y_bin_middles: Option<Vec<f64>> = None;

        // For continuous data, we need to apply binning
        let processed_df = {
            let mut df = self.data.df.clone();

            // Handle x data
            if !x_is_discrete {
                // Get the current x_series from the DataFrame
                let current_x_series = df.column(&x_encoding.field)?.f64()?.clone().into_series();

                // Calculate number of bins - use explicit value if set, otherwise:
                // - If all values are the same, use 1 bin
                // - Otherwise use square root rule
                let unique_count = current_x_series.n_unique()?;
                let n_bins = if unique_count == 1 {
                    1
                } else {
                    x_encoding
                        .bins
                        .unwrap_or_else(|| ((unique_count as f64).sqrt() as usize).clamp(5, 50))
                };

                // Get min and max values for binning using Polars' built-in methods
                let min_val = current_x_series.f64()?.min().expect(
                    "Internal error: Failed to calculate minimum value for rect chart data",
                );
                let max_val = current_x_series.f64()?.max().expect(
                    "Internal error: Failed to calculate maximum value for rect chart data",
                );

                // Create bins
                let bin_width = if n_bins > 1 {
                    (max_val - min_val) / (n_bins as f64)
                } else {
                    1.0 // arbitrary non-zero value when n_bins = 1
                };

                let mut bins = Vec::with_capacity(n_bins + 1);
                for i in 0..=n_bins {
                    bins.push(min_val + (i as f64) * bin_width);
                }

                // Store bin labels and middle values
                let labels: Vec<String> = (0..n_bins).map(|i| format!("bin_{}", i)).collect();
                x_bin_labels = Some(labels.clone());
                let middles: Vec<f64> = bins.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect();
                x_bin_middles = Some(middles.clone());

                // Create binned column
                let binned_series =
                    crate::stats::stat_binning::cut(&current_x_series, &bins, &labels);

                df = df
                    .lazy()
                    .with_column(lit(binned_series).alias(&x_encoding.field))
                    .collect()?;
            }

            // Handle y data
            if !y_is_discrete {
                // Get the current y_series from the DataFrame
                let current_y_series = df.column(&y_encoding.field)?.f64()?.clone().into_series();

                // Calculate number of bins - use explicit value if set, otherwise:
                // - If all values are the same, use 1 bin
                // - Otherwise use square root rule
                let unique_count = current_y_series.n_unique()?;
                let n_bins = if unique_count == 1 {
                    1
                } else {
                    y_encoding
                        .bins
                        .unwrap_or_else(|| ((unique_count as f64).sqrt() as usize).clamp(5, 50))
                };

                // Get min and max values for binning using Polars' built-in methods
                let min_val = current_y_series.f64()?.min().expect(
                    "Internal error: Failed to calculate minimum value for rect chart data",
                );
                let max_val = current_y_series.f64()?.max().expect(
                    "Internal error: Failed to calculate maximum value for rect chart data",
                );

                // Create bins
                let bin_width = if n_bins > 1 {
                    (max_val - min_val) / (n_bins as f64)
                } else {
                    1.0 // arbitrary non-zero value when n_bins = 1
                };

                let mut bins = Vec::with_capacity(n_bins + 1);
                for i in 0..=n_bins {
                    bins.push(min_val + (i as f64) * bin_width);
                }

                // Store bin labels and middle values
                let labels: Vec<String> = (0..n_bins).map(|i| format!("bin_{}", i)).collect();
                y_bin_labels = Some(labels.clone());
                let middles: Vec<f64> = bins.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect();
                y_bin_middles = Some(middles.clone());

                // Create binned column
                let binned_series =
                    crate::stats::stat_binning::cut(&current_y_series, &bins, &labels);

                df = df
                    .lazy()
                    .with_column(lit(binned_series).alias(&y_encoding.field))
                    .collect()?;
            }

            df
        };

        // Since we now require color encoding, we always sum the color values when they share the same x,y coordinates
        let grouped_df = processed_df
            .lazy()
            .group_by_stable([col(&x_encoding.field), col(&y_encoding.field)])
            .agg([col(&color_encoding.field)
                .sum()
                .alias(&color_encoding.field)])
            .collect()?;

        // Fill in missing bin combinations with 0 values if we have binned data
        let filled_df = if x_bin_labels.is_some() || y_bin_labels.is_some() {
            // Get all possible bin labels
            let all_x_bins = x_bin_labels.clone().unwrap_or_else(|| {
                // For discrete data, get unique values
                grouped_df
                    .column(&x_encoding.field)
                    .expect("x column should exist")
                    .unique_stable()
                    .expect("should be able to get unique values")
                    .str()
                    .expect("should be string type")
                    .into_no_null_iter()
                    .map(|s| s.to_string())
                    .collect()
            });

            let all_y_bins = y_bin_labels.clone().unwrap_or_else(|| {
                // For discrete data, get unique values
                grouped_df
                    .column(&y_encoding.field)
                    .expect("y column should exist")
                    .unique_stable()
                    .expect("should be able to get unique values")
                    .str()
                    .expect("should be string type")
                    .into_no_null_iter()
                    .map(|s| s.to_string())
                    .collect()
            });

            // Create all combinations of x and y bins
            let x_repeated: Vec<String> = all_x_bins
                .iter()
                .flat_map(|x| vec![x.clone(); all_y_bins.len()])
                .collect();

            let y_repeated: Vec<String> = all_y_bins
                .iter()
                .cycle()
                .take(all_x_bins.len() * all_y_bins.len())
                .cloned()
                .collect();

            // Create DataFrame with all combinations
            let all_combinations_df = df![
                &x_encoding.field => x_repeated,
                &y_encoding.field => y_repeated
            ]?;

            // Join with the grouped data to fill in missing combinations
            let joined_df = all_combinations_df
                .lazy()
                .join(
                    grouped_df.lazy(),
                    [col(&x_encoding.field), col(&y_encoding.field)],
                    [col(&x_encoding.field), col(&y_encoding.field)],
                    JoinType::Left.into(),
                )
                .collect()?;

            // Fill null color values with 0
            joined_df
                .lazy()
                .with_column(col(&color_encoding.field).fill_null(lit(0)))
                .collect()?
        } else {
            grouped_df
        };

        // Convert bin labels to middle values for continuous data
        let final_df = {
            let mut df = filled_df.clone();

            // Replace x bin labels with middle values if x-axis was binned
            if let (Some(labels), Some(middles)) = (&x_bin_labels, &x_bin_middles) {
                // Create mapping from labels to middle values
                let mut label_to_middle = std::collections::HashMap::new();
                for (label, &middle) in labels.iter().zip(middles.iter()) {
                    label_to_middle.insert(label.clone(), middle);
                }

                // Map the x column values
                let x_series = df
                    .column(&x_encoding.field)
                    .unwrap_or_else(|e| {
                        panic!(
                            "Internal error: x column '{}' should exist: {:?}",
                            x_encoding.field, e
                        )
                    })
                    .str()
                    .unwrap_or_else(|e| {
                        panic!(
                            "Internal error: x column '{}' should be string type: {:?}",
                            x_encoding.field, e
                        )
                    });
                let new_x_values: Vec<Option<f64>> = x_series
                    .into_iter()
                    .map(|opt_val| opt_val.and_then(|val| label_to_middle.get(val).copied()))
                    .collect();

                let new_x_series = Series::new((&x_encoding.field).into(), new_x_values);
                df = df
                    .lazy()
                    .with_column(lit(new_x_series).alias(&x_encoding.field))
                    .collect()?;
            }

            // Replace y bin labels with middle values if y-axis was binned
            if let (Some(labels), Some(middles)) = (&y_bin_labels, &y_bin_middles) {
                // Create mapping from labels to middle values
                let mut label_to_middle = std::collections::HashMap::new();
                for (label, &middle) in labels.iter().zip(middles.iter()) {
                    label_to_middle.insert(label.clone(), middle);
                }

                // Map the y column values
                let y_series = df
                    .column(&y_encoding.field)
                    .unwrap_or_else(|e| {
                        panic!(
                            "Internal error: y column '{}' should exist: {:?}",
                            y_encoding.field, e
                        )
                    })
                    .str()
                    .unwrap_or_else(|e| {
                        panic!(
                            "Internal error: y column '{}' should be string type: {:?}",
                            y_encoding.field, e
                        )
                    });
                let new_y_values: Vec<Option<f64>> = y_series
                    .into_iter()
                    .map(|opt_val| opt_val.and_then(|val| label_to_middle.get(val).copied()))
                    .collect();

                let new_y_series = Series::new((&y_encoding.field).into(), new_y_values);
                df = df
                    .lazy()
                    .with_column(lit(new_y_series).alias(&y_encoding.field))
                    .collect()?;
            }

            df
        };

        self.data = (&final_df).try_into()?;
        Ok(self)
    }
}
