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
        // --- STEP 0: Capture Encoding and Schema Metadata ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or(ChartonError::Encoding("X encoding missing".into()))?;
        let x_field = x_enc.field.as_str();

        // Store the original DataType of the X-axis.
        // If it's Temporal (Date/Datetime), we will temporarily cast it to Int64
        // to prevent Polars SchemaMismatch errors during join and stacking operations.
        let original_x_dtype = self.data.df.column(x_field)?.dtype().clone();
        let is_temporal = original_x_dtype.is_temporal();

        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or(ChartonError::Encoding("Y encoding missing".into()))?;
        let mode = &y_enc.stack;
        let y_field = y_enc.field.as_str();
        let color_enc_opt = self.encoding.color.as_ref();

        let y0 = format!("{}_{}_min", TEMP_SUFFIX, y_field);
        let y1 = format!("{}_{}_max", TEMP_SUFFIX, y_field);
        let total = format!("{}_{}_total", TEMP_SUFFIX, y_field);

        // Initial lazy frame for transformations
        let mut lazy_df = self.data.df.clone().lazy();

        // --- STEP 1: Type Masking (Temporal to Int64) ---
        // Masking allows mathematical operations like cumulative sums and joins
        // to treat timestamps as simple numeric nanoseconds.
        if is_temporal {
            lazy_df = lazy_df.with_column(col(x_field).cast(DataType::Int64));
        }

        // --- STEP 2: Handle Unstacked Mode (StackMode::None) ---
        if matches!(mode, StackMode::None) {
            let mut final_lazy = lazy_df
                .with_column(lit(0.0).alias(&y0))
                .with_column(col(y_field).alias(&y1));

            // Restore original type before finishing
            if is_temporal {
                final_lazy = final_lazy.with_column(col(x_field).cast(original_x_dtype));
            }

            self.data.df = final_lazy.collect()?;
            return Ok(self);
        }

        // --- STEP 3: Capture Color Appearance Order ---
        // Ensures the stacking order matches the data order rather than alphabetical order.
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

        // --- STEP 4: Data Imputation (Cartesian Product Gap Filling) ---
        // Ensures visual continuity by adding 0.0 values where data is missing for specific groups.
        if let Some(ce) = &color_enc_opt {
            let c_field = &ce.field;

            // Since x_field is now Int64, these unique values are safe to clone into a grid.
            let x_uniques = lazy_df
                .clone()
                .select([col(x_field)])
                .unique_stable(None, UniqueKeepStrategy::First)
                .collect()?
                .column(x_field)?
                .clone();

            let c_uniques = self.data.df.column(c_field)?.unique_stable()?;

            let x_len = x_uniques.len();
            let c_len = c_uniques.len();

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
            ]?
            .lazy();

            // Perform Left Join and fill missing Y values with zero.
            lazy_df = grid_df
                .join(
                    lazy_df,
                    [col(x_field), col(c_field)],
                    [col(x_field), col(c_field)],
                    JoinType::Left.into(),
                )
                .with_column(col(y_field).fill_null(lit(0.0)));
        }

        // --- STEP 5: Sort and Grouped Stacking ---
        // Sort primarily by X and secondarily by the original color order.
        if let Some((c_field, order_col, order_df)) = &color_order_df {
            lazy_df = lazy_df
                .join(
                    order_df.clone(),
                    [col(*c_field)],
                    [col(*c_field)],
                    JoinType::Left.into(),
                )
                .sort_by_exprs(
                    [col(x_field), col(order_col)],
                    SortMultipleOptions::default().with_maintain_order(true),
                )
                .drop([col(order_col)]);
        } else {
            lazy_df = lazy_df.sort_by_exprs([col(x_field)], SortMultipleOptions::default());
        }

        // --- STEP 6: Calculate Cumulative Boundaries (y0, y1) ---
        // y1 is the running total; y0 is the previous total (the baseline).
        lazy_df = lazy_df.with_column(col(y_field).cum_sum(false).over([col(x_field)]).alias(&y1));
        lazy_df = lazy_df.with_column(
            col(&y1)
                .shift(lit(1))
                .over([col(x_field)])
                .fill_null(lit(0.0))
                .alias(&y0),
        );

        // --- STEP 7: Apply Normalization or Centering (Streamgraph) ---
        if matches!(mode, StackMode::Normalize | StackMode::Center) {
            lazy_df = lazy_df.with_column(col(&y1).max().over([col(x_field)]).alias(&total));

            if matches!(mode, StackMode::Normalize) {
                lazy_df = lazy_df.with_column((col(&y0) / col(&total)).alias(&y0));
                lazy_df = lazy_df.with_column((col(&y1) / col(&total)).alias(&y1));
            } else if matches!(mode, StackMode::Center) {
                lazy_df = lazy_df.with_column((col(&y0) - col(&total) / lit(2.0)).alias(&y0));
                lazy_df = lazy_df.with_column((col(&y1) - col(&total) / lit(2.0)).alias(&y1));
            }
            lazy_df = lazy_df.drop([total]);
        }

        // --- STEP 8: Finalize and Restore Schema ---
        // Restore the original temporal type (e.g., Datetime[ns]) so that
        // the TemporalScale can correctly format axis ticks.
        if is_temporal {
            lazy_df = lazy_df.with_column(col(x_field).cast(original_x_dtype));
        }

        self.data.df = lazy_df.collect()?;
        self.data = (&self.data.df).into_source()?;

        Ok(self)
    }
}
