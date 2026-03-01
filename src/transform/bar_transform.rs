use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::prelude::IntoChartonSource;
use polars::prelude::*;

impl<T: Mark> Chart<T> {
    /// Consolidates and prepares data for Bar-like marks (Bar, Rose, Boxplot).
    ///
    /// This transformation follows a "Data-Driven Layout" strategy:
    /// 1. **Deduplication**: If X and Color use the same field, we group only once
    ///    to prevent Polars errors and signal a "Self-Mapping" layout (full width).
    /// 2. **Aggregation**: Computes the mean for the Y-axis value.
    /// 3. **Gap Filling**: Uses a Cartesian Product to ensure every X-category has
    ///    the same number of rows (filling missing combinations with 0).
    ///    This ensures that grouped bars have consistent widths and alignments.
    pub(crate) fn transform_bar_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encoding Context ---
        // Basic requirement: X and Y must exist. Color is optional.
        // Get mutable references so we can modify properties like 'stack' for Pie charts.
        let y_enc = self.encoding.y.as_mut().unwrap();
        let x_enc = self.encoding.x.as_ref().unwrap();
        let color_enc_opt = self.encoding.color.as_ref();

        let x_field = &x_enc.field;
        let y_field = &y_enc.field;

        // --- NEW: PIE/SINGLE-AXIS MODE HANDLING ---
        // If x_field is an empty string, it signifies a single-axis layout (Pie Chart).
        // 1. Force 'stack' to true: Essential for pie slices to chain head-to-tail.
        // 2. Inject virtual column: Ensure Polars can find the "" column for grouping.
        if x_field.is_empty() {
            y_enc.stack = true;

            if !self
                .data
                .df
                .get_column_names()
                .contains(&&PlSmallStr::from_static(""))
            {
                self.data.df = self
                    .data
                    .df
                    .clone()
                    .lazy()
                    .with_column(lit("").alias(""))
                    .collect()?;
            }
        }

        // --- STEP 2: Aggregation & Grouping ---
        // We define the grouping strategy based on field overlap.
        let grouped_df = if let Some(ce) = color_enc_opt {
            let mut group_selectors = vec![col(x_field)];

            // Deduplication Logic:
            // If Color is the same as X, it's an "Aesthetic Mapping" (just coloring).
            // If Color is different, it's a "Grouping Mapping" (Dodge/side-by-side).
            if &ce.field != x_field {
                group_selectors.push(col(&ce.field));
            }

            self.data
                .df
                .clone()
                .lazy()
                .group_by_stable(group_selectors)
                .agg([col(y_field).sum().alias(y_field)])
                .collect()?
        } else {
            // Simple case: No color mapping, group by X only.
            self.data
                .df
                .clone()
                .lazy()
                .group_by_stable([col(x_field)])
                .agg([col(y_field).sum().alias(y_field)])
                .collect()?
        };

        // --- STEP 3: Normalization (Optional) ---
        // If 'normalize' is true, values are converted to proportions (0.0 - 1.0)
        // relative to the total sum of their specific X group.
        let grouped_df = if y_enc.normalize {
            grouped_df
                .lazy()
                .with_column(
                    (col(y_field).cast(DataType::Float64)
                        / col(y_field).sum().over([col(x_field)]))
                    .alias(y_field),
                )
                .collect()?
        } else {
            grouped_df
        };

        // --- STEP 4: Cartesian Product Gap Filling ---
        // This is critical for the "Row-Count Driven Layout".
        // We ensure every X group has exactly the same number of rows so the
        // Renderer can calculate bar widths and offsets consistently.
        let filled_df = if let Some(ce) = color_enc_opt {
            // If X and Color are the same field, the mapping is 1:1.
            // Every group already has exactly 1 row. No filling required.
            if &ce.field == x_field {
                grouped_df
            } else {
                // Determine the unique set of categories for both dimensions.
                // We use unique_stable to preserve user-defined data order.
                let x_uniques = grouped_df.column(x_field)?.unique_stable()?;
                let c_uniques = grouped_df.column(&ce.field)?.unique_stable()?;

                let x_len = x_uniques.len();
                let c_len = c_uniques.len();

                // Build a "Grid" of all possible X + Color combinations.
                let mut x_repeated = Vec::with_capacity(x_len * c_len);
                let mut c_repeated = Vec::with_capacity(x_len * c_len);

                for i in 0..x_len {
                    let x_val = x_uniques.get(i)?;
                    for j in 0..c_len {
                        x_repeated.push(x_val.clone());
                        c_repeated.push(c_uniques.get(j)?.clone());
                    }
                }

                let all_combos = df![
                    x_field => x_repeated,
                    &ce.field => c_repeated
                ]?;

                // Left Join the grid with our data.
                // Any missing combination (gap) will result in a Null value.
                all_combos
                    .lazy()
                    .join(
                        grouped_df.lazy(),
                        [col(x_field), col(&ce.field)],
                        [col(x_field), col(&ce.field)],
                        JoinType::Left.into(),
                    )
                    // Convert Nulls to 0. These rows act as "Invisible Spacers"
                    // to maintain correct bar positioning in grouped charts.
                    .with_column(col(y_field).fill_null(lit(0)))
                    .collect()?
            }
        } else {
            grouped_df
        };

        // Final Step: Update the chart's data source with the clean, expanded DataFrame.
        self.data = (&filled_df).into_source()?;

        Ok(self)
    }
}
