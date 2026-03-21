use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::encode::y::StackMode;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::prelude::IntoChartonSource;
use polars::prelude::*;

impl<T: Mark> Chart<T> {
    /// Prepares data for area charts based on the configured StackMode.
    ///
    /// This transformation adds `y0` (baseline) and `y1` (top) columns to the DataFrame.
    /// The renderer uses these columns instead of calculating baselines at render time.
    ///
    /// # Stack Modes
    ///
    /// | Mode | y0 | y1 |
    /// |------|----|----|
    /// | `None` | 0 | raw value |
    /// | `Stacked` | cumulative sum (previous) | cumulative sum (current) |
    /// | `Normalize` | normalized cumulative (previous) | normalized cumulative (current) |
    /// | `Center` | centered cumulative (previous) | centered cumulative (current) |
    ///
    /// # Key Features
    ///
    /// 1. **Data Imputation**: Automatically fills missing X values for each color group
    /// 2. **Stable Stacking Order**: Preserves color appearance order (not alphabetical)
    /// 3. **Renderer Ready**: Outputs y0/y1 columns for efficient GPU rendering
    pub(crate) fn transform_area_data(mut self) -> Result<Self, ChartonError> {
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or(ChartonError::Encoding("Y encoding missing".into()))?;
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or(ChartonError::Encoding("X encoding missing".into()))?;

        let mode = &y_enc.stack;
        let y_field = y_enc.field.as_str();
        let x_field = x_enc.field.as_str();
        let color_enc_opt = self.encoding.color.as_ref();

        let y0 = format!("{}_{}_min", TEMP_SUFFIX, y_field);
        let y1 = format!("{}_{}_max", TEMP_SUFFIX, y_field);
        let total = format!("{}_{}_total", TEMP_SUFFIX, y_field);

        // ========================================================================
        // STACK MODE: NONE (Unstacked/Overlay)
        // ========================================================================
        // No stacking transformation needed, but still create y0/y1 for renderer consistency.
        // Each area draws independently from the zero baseline.
        if matches!(mode, StackMode::None) {
            self.data.df = self
                .data
                .df
                .clone()
                .lazy()
                .with_column(lit(0.0).alias(y0))
                .with_column(col(y_field).alias(y1))
                .collect()?;
            return Ok(self);
        }

        // ========================================================================
        // STACK MODE: Stacked, Normalize, or Center
        // ========================================================================
        // These modes require data alignment and cumulative sum calculations.
        let mut lazy_df = self.data.df.clone().lazy();

        // --- STEP 1: CAPTURE COLOR APPEARANCE ORDER ---
        // Preserve the original order in which colors appear in the data.
        // This ensures consistent stacking order (first seen = bottom layer).
        let color_order_df = if let Some(ce) = &color_enc_opt {
            let order_col = format!("{}_order", crate::TEMP_SUFFIX);
            let c_field = &ce.field;
            let df = self
                .data
                .df
                .clone()
                .lazy()
                .select([col(c_field)])
                .unique_stable(None, UniqueKeepStrategy::First)
                .with_row_index(&order_col, None);
            Some((c_field, order_col, df))
        } else {
            None
        };

        // --- STEP 2: DATA IMPUTATION (Cartesian Product Gap Filling) ---
        // Ensures every color group has data points at every X position.
        // Missing values are filled with 0.0 to maintain visual continuity.
        if let Some(ce) = &color_enc_opt {
            let c_field = &ce.field;

            // Get unique X values (preserve data order)
            let x_uniques = self.data.df.column(x_field)?.unique_stable()?;
            // Get unique Color values (preserve data order)
            let c_uniques = self.data.df.column(c_field)?.unique_stable()?;

            let x_len = x_uniques.len();
            let c_len = c_uniques.len();

            // Build Cartesian product grid (all X × Color combinations)
            let mut x_repeated = Vec::with_capacity(x_len * c_len);
            let mut c_repeated = Vec::with_capacity(x_len * c_len);

            for i in 0..x_len {
                let x_val = x_uniques.get(i)?;
                for j in 0..c_len {
                    x_repeated.push(x_val.clone());
                    c_repeated.push(c_uniques.get(j)?.clone());
                }
            }

            let grid_df = df![
                x_field => x_repeated,
                c_field => c_repeated
            ]?;

            // Left join grid with original data, fill missing Y values with 0.0
            lazy_df = grid_df
                .lazy()
                .join(
                    lazy_df,
                    [col(x_field), col(c_field)],
                    [col(x_field), col(c_field)],
                    JoinType::Left.into(),
                )
                .with_column(col(y_field).fill_null(lit(0.0)));
        }

        // --- STEP 3: SORT BY X THEN COLOR ORDER ---
        // First sort by X axis (ascending)
        lazy_df = lazy_df.sort_by_exprs([col(x_field)], SortMultipleOptions::default());

        // Then sort by color appearance order (for stable stacking)
        if let Some((c_field, order_col, order_df)) = color_order_df {
            lazy_df = lazy_df
                .join(
                    order_df,
                    [col(c_field)],
                    [col(c_field)],
                    JoinType::Left.into(),
                )
                .sort_by_exprs([col(&order_col)], SortMultipleOptions::default())
                .drop([order_col]);
        }

        // --- STEP 4: CALCULATE CUMULATIVE SUM (y1) ---
        // For each X position, calculate the running total across color groups.
        // This determines the top boundary of each stacked area.
        lazy_df = lazy_df.with_column(col(y_field).cum_sum(false).over([col(x_field)]).alias(&y1));

        // --- STEP 5: CALCULATE BASELINE (y0) ---
        // The baseline for each area is the cumulative sum of previous groups.
        // Use shift(1) to get the previous group's cumulative sum.
        lazy_df = lazy_df.with_column(
            col(&y1)
                .shift(lit(1))
                .over([col(x_field)])
                .fill_null(lit(0.0))
                .alias(&y0),
        );

        // --- STEP 6: APPLY NORMALIZATION OR CENTERING ---
        if matches!(mode, StackMode::Normalize | StackMode::Center) {
            // Calculate total height per X (max y1 in each group = total sum)
            lazy_df = lazy_df.with_column(col(&y1).max().over([col(x_field)]).alias(&total));

            if matches!(mode, StackMode::Normalize) {
                // Normalize: divide by total so each X slice sums to 1.0 (100% stacked)
                lazy_df = lazy_df.with_column((col(&y0) / col(&total)).alias(&y0));
                lazy_df = lazy_df.with_column((col(&y1) / col(&total)).alias(&y1));
            } else if matches!(mode, StackMode::Center) {
                // Center: offset by -0.5 * total to center around zero (streamgraph)
                lazy_df = lazy_df.with_column((col(&y0) - col(&total) / lit(2.0)).alias(&y0));
                lazy_df = lazy_df.with_column((col(&y1) - col(&total) / lit(2.0)).alias(&y1));
            }

            // Drop temporary total column
            lazy_df = lazy_df.drop([total]);
        }

        // --- STEP 7: FINALIZE DATA ---
        self.data.df = lazy_df.collect()?;
        self.data = (&self.data.df).into_source()?;

        println!("{}", self.data.df);
        Ok(self)
    }
}
