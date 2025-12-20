use crate::chart::common::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use polars::prelude::*;

impl<T: Mark> Chart<T> {
    // Handle grouping and aggregation of data for histogram chart
    pub(crate) fn transform_histogram_data(mut self) -> Result<Self, ChartonError> {
        // Check if we have the required encodings
        let x_encoding = self.encoding.x.as_ref().unwrap();
        let y_encoding = self.encoding.y.as_ref().unwrap();

        // Now perform the data transformation
        let (bin_field, count_field) = (x_encoding.field.clone(), y_encoding.field.clone());
        // Handle continuous data by binning
        let processed_df = {
            let mut df = self.data.df.clone();

            // Get the x series data (already converted to f64)
            let x_series = df.column(&bin_field)?.f64()?.clone().into_series();

            // Get unique count using Polars' built-in method
            let unique_count = x_series.n_unique()?;

            // Calculate number of bins - use explicit value if set, otherwise:
            // - If all values are the same, use 1 bin
            // - Otherwise use square root rule
            let n_bins = if unique_count == 1 {
                1
            } else {
                x_encoding
                    .bins
                    .unwrap_or_else(|| ((unique_count as f64).sqrt() as usize).max(5).min(50))
            };

            // Get min and max values for binning using Polars' built-in methods
            let min_val = x_series
                .f64()?
                .min()
                .expect("Internal error: Failed to calculate minimum value for histogram data");
            let max_val = x_series
                .f64()?
                .max()
                .expect("Internal error: Failed to calculate maximum value for histogram data");

            // Create bins. bin_width is used to calculate the data range of each bin
            let bin_width = if n_bins > 1 {
                (max_val - min_val) / (n_bins as f64)
            } else {
                1.0 // arbitrary non-zero value when n_bins = 1
            };

            let mut bins = Vec::with_capacity(n_bins + 1);
            for i in 0..=n_bins {
                bins.push(min_val + (i as f64) * bin_width);
            }

            // Store bin labels for later use
            let labels: Vec<String> = (0..n_bins).map(|i| format!("bin_{}", i)).collect();

            // Calculate middle values of bins
            let middles: Vec<f64> = bins.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect();

            // Create binned column
            let binned_series = crate::stats::stat_binning::cut(&x_series, &bins, &labels);
            let renamed_series = binned_series.with_name((&bin_field).into());
            df.with_column(renamed_series)?;

            // Group by bins and count occurrences, using count_field as the column name
            // Handle color encoding similar to bar charts
            let grouped_df = if let Some(color_encoding) = &self.encoding.color {
                // If we have color encoding, group by both bin field and color field
                df.lazy()
                    .group_by_stable([col(&bin_field), col(&color_encoding.field)])
                    .agg([col(&bin_field).count().alias(&count_field)])
                    .collect()?
            } else {
                // If no color encoding, group by bin field only
                df.lazy()
                    .group_by_stable([col(&bin_field)])
                    .agg([col(&bin_field).count().alias(&count_field)])
                    .collect()?
            };

            // Apply normalization if requested
            let grouped_df = if y_encoding.normalize {
                if let Some(color_encoding) = &self.encoding.color {
                    // Normalize within each color group (each group sums to 1)
                    grouped_df
                        .lazy()
                        .with_column(
                            (col(&count_field).cast(DataType::Float64)
                                / col(&count_field).sum().over([col(&color_encoding.field)]))
                            .alias(&count_field),
                        )
                        .collect()?
                } else {
                    // Normalize all values to sum to 1
                    grouped_df
                        .lazy()
                        .with_column(
                            (col(&count_field).cast(DataType::Float64) / col(&count_field).sum())
                                .alias(&count_field),
                        )
                        .collect()?
                }
            } else {
                grouped_df
                    .lazy()
                    .with_column(col(&count_field).cast(DataType::Float64))
                    .collect()?
            };

            // Create all possible bin labels to ensure empty bins are included
            let all_bin_labels: Vec<String> = (0..n_bins).map(|i| format!("bin_{}", i)).collect();

            // Handle color encoding when filling missing combinations
            let filled_df = if let Some(color_encoding) = &self.encoding.color {
                // Get unique color values
                let color_unique_series =
                    grouped_df.column(&color_encoding.field)?.unique_stable()?;
                let color_values: Vec<String> = color_unique_series
                    .str()?
                    .into_no_null_iter()
                    .map(|s| s.to_string())
                    .collect();

                // Create all combinations of bin labels and color values
                let bin_repeated: Vec<String> = all_bin_labels
                    .iter()
                    .flat_map(|bin| vec![bin.clone(); color_values.len()])
                    .collect();

                let color_repeated: Vec<String> = color_values
                    .iter()
                    .cycle()
                    .take(all_bin_labels.len() * color_values.len()).cloned()
                    .collect();

                // Create DataFrame with all combinations
                let all_combinations_df = df![
                    &bin_field => bin_repeated,
                    &color_encoding.field => color_repeated
                ]?;

                // Join with the grouped data to fill in missing combinations
                

                all_combinations_df
                    .lazy()
                    .join(
                        grouped_df.lazy(),
                        [col(&bin_field), col(&color_encoding.field)],
                        [col(&bin_field), col(&color_encoding.field)],
                        JoinType::Left.into(),
                    )
                    .collect()?
                    .lazy()
                    .with_column(col(&count_field).fill_null(lit(0)))
                    .collect()?
            } else {
                // Create DataFrame with all bins for no color encoding case
                let all_bins_df = df![
                    &bin_field => all_bin_labels
                ]?;

                // Join with the grouped data to include zero counts for empty bins
                

                all_bins_df
                    .lazy()
                    .join(
                        grouped_df.lazy(),
                        [col(&bin_field)],
                        [col(&bin_field)],
                        JoinType::Left.into(),
                    )
                    .collect()?
                    .lazy()
                    .with_column(col(&count_field).fill_null(lit(0)))
                    .collect()?
            };

            // Replace bin labels with middle values
            let mut label_to_middle = std::collections::HashMap::new();
            for (label, &middle) in (0..n_bins)
                .map(|i| format!("bin_{}", i))
                .zip(middles.iter())
            {
                label_to_middle.insert(label, middle);
            }

            // Map the bin column values to middle values
            let bin_series = filled_df
                .column(&bin_field)?
                .str()
                .expect("Bin field should be string type");
            let new_bin_values: Vec<Option<f64>> = bin_series
                .into_iter()
                .map(|opt_val| opt_val.and_then(|val| label_to_middle.get(val).copied()))
                .collect();

            let new_bin_series = Series::new((&bin_field).into(), new_bin_values);
            let mut result_df = filled_df;
            // Replace the column (e.g. bin_field) while maintaining column order
            // with_column will replace the existing column with the same name
            result_df.with_column(new_bin_series)?;
            result_df
        };

        self.data = (&processed_df).try_into()?;
        Ok(self)
    }
}
