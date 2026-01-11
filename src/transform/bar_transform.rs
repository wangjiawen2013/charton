use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use polars::prelude::*;

impl<T: Mark> Chart<T> {
    // Handle grouping and aggregation of data for bar chart
    pub(crate) fn transform_bar_data(mut self) -> Result<Self, ChartonError> {
        // Check if we have the required encodings
        let x_encoding = self.encoding.x.as_ref().unwrap();
        let y_encoding = self.encoding.y.as_ref().unwrap();

        // Determine grouping and value columns - no longer based on orientation
        let (group_col, value_col) = (&x_encoding.field, &y_encoding.field);

        // Group by the discrete axis and color if present
        let grouped_df = if let Some(color_encoding) = &self.encoding.color {
            // If we have color encoding, group by the discrete axis and color
            self.data
                .df
                .clone()
                .lazy()
                .group_by_stable([col(group_col), col(&color_encoding.field)])
                .agg([col(value_col).mean().alias(value_col)])
                .collect()?
        } else {
            // If no color encoding, group by the discrete axis only
            self.data
                .df
                .clone()
                .lazy()
                .group_by_stable([col(group_col)])
                .agg([col(value_col).mean().alias(value_col)])
                .collect()?
        };

        // Apply normalization if requested
        let grouped_df = if y_encoding.normalize {
            // Normalize within each group (each group sums to 1)
            grouped_df
                .lazy()
                .with_column(
                    (col(value_col).cast(DataType::Float64)
                        / col(value_col).sum().over([col(group_col)]))
                    .alias(value_col),
                )
                .collect()?
        } else {
            grouped_df
        };

        // If we have color encoding, ensure all combinations of group and color exist
        let filled_df = if let Some(color_encoding) = &self.encoding.color {
            // Get unique values for each dimension
            let group_unique_series = grouped_df.column(group_col)?.unique_stable()?;
            let color_unique_series = grouped_df.column(&color_encoding.field)?.unique_stable()?;

            // Get the count of unique values for each dimension
            let group_count = group_unique_series.len();
            let color_count = color_unique_series.len();

            // Create all combinations by repeating values appropriately
            // For group values: repeat each group value `color_count` times
            let mut group_repeated = Vec::new();
            for i in 0..group_count {
                let val = group_unique_series.get(i)?;
                for _ in 0..color_count {
                    group_repeated.push(val.clone());
                }
            }

            // For color values: cycle through all color values `group_count` times
            let mut color_repeated = Vec::new();
            for _ in 0..group_count {
                for i in 0..color_count {
                    color_repeated.push(color_unique_series.get(i)?.clone());
                }
            }

            // Create DataFrame with all combinations
            let all_combinations_df = df![
                group_col => group_repeated,
                &color_encoding.field => color_repeated
            ]?;

            // Join with the grouped data to fill in missing combinations
            let joined_df = all_combinations_df
                .lazy()
                .join(
                    grouped_df.lazy(),
                    [col(group_col), col(&color_encoding.field)],
                    [col(group_col), col(&color_encoding.field)],
                    JoinType::Left.into(),
                )
                .collect()?;

            // Fill null value columns with 0
            joined_df
                .lazy()
                .with_column(col(value_col).fill_null(lit(0)))
                .collect()?
        } else {
            grouped_df
        };

        self.data = (&filled_df).try_into()?;

        Ok(self)
    }
}
