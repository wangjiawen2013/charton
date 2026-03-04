use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::prelude::IntoChartonSource;
use polars::prelude::*;

impl<T: Mark> Chart<T> {
    /// Prepares data for Error Bar marks by calculating statistical intervals.
    ///
    /// This transformation follows the "Data-Driven Layout" strategy used in Bar charts:
    /// 1. **Deduplication**: Checks if Color field is the same as X to determine grouping.
    /// 2. **Aggregation**: Computes Mean and Standard Deviation (1-sigma) for the Y-axis.
    /// 3. **Gap Filling**: Uses a Cartesian Product to ensure every X-category has
    ///    the same number of groups, preventing misalignment in grouped layouts.
    pub(crate) fn transform_errorbar_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encoding Context ---
        let y_enc = self.encoding.y.as_ref().unwrap();
        let x_enc = self.encoding.x.as_ref().unwrap();
        // 这儿针对的是一个x和color（如果有）多个x值的情况，如果x,y, standard都已经算好了，则用calculate_transform.rs
        let color_enc_opt = self.encoding.color.as_ref(); // 提前在chart中判断是否为离散型，需要离散型

        let x_field = &x_enc.field;
        let y_field = &y_enc.field;
        
        // Define temp column names for the interval bounds
        let y_min_col = format!("{}_{}_min", TEMP_SUFFIX, y_field);
        let y_max_col = format!("{}_{}_max", TEMP_SUFFIX, y_field);

        // --- STEP 2: Aggregation & Grouping ---
        let mut group_selectors = vec![col(x_field)];
        
        // Only add Color to grouping if it's a different field than X
        let has_grouping_color = if let Some(ce) = color_enc_opt {
            if &ce.field != x_field {
                group_selectors.push(col(&ce.field));
                true
            } else { false }
        } else { false };

        let grouped_df = self.data.df.clone()
            .lazy()
            .group_by_stable(group_selectors)
            .agg([
                // Center point
                col(y_field).mean().alias(y_field),
                // Standard Deviation Interval (Sample std, ddof=1). n=1 results in Null here.
                (col(y_field).mean() - col(y_field).std(1)).alias(&y_min_col),
                (col(y_field).mean() + col(y_field).std(1)).alias(&y_max_col),
            ])
            .collect()?;

        // --- STEP 3: Cartesian Product Gap Filling ---
        // Ensuring structural consistency across all X categories
        let filled_df = if has_grouping_color {
            let ce = color_enc_opt.unwrap();
            
            // preserved user-defined order
            let x_uniques = grouped_df.column(x_field)?.unique_stable()?;
            let c_uniques = grouped_df.column(&ce.field)?.unique_stable()?;

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

            let all_combinations = df![
                x_field => x_repeated,
                &ce.field => c_repeated
            ]?;

            all_combinations.lazy()
                .join(
                    grouped_df.lazy(),
                    [col(x_field), col(&ce.field)],
                    [col(x_field), col(&ce.field)],
                    JoinType::Left.into(),
                )
                // Note: We leave Y values as Null for missing gaps.
                // The ErrorBar renderer should 'continue' on Nulls rather than draw a 0-length bar.
                .collect()?
        } else {
            grouped_df
        };

        // Final Step: Update the chart's data source
        self.data = (&filled_df).into_source()?;

        Ok(self)
    }
}