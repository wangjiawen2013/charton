use crate::core::layer::{MarkRenderer, RenderBackend, RectConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::mark::histogram::MarkHist;
use crate::visual::color::SingleColor;
use crate::error::ChartonError;
use crate::Precision;
use polars::prelude::*;

/// Implementation of `MarkRenderer` for Histogram charts.
/// 
/// This renderer consumes the DataFrame pre-processed by `transform_histogram_data`,
/// where X is already binned (mapped to bin middles) and Y is the calculated frequency.
impl MarkRenderer for Chart<MarkHist> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data.df;
        if df_source.height() == 0 {
            return Ok(());
        }

        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkHist configuration is missing".into()))?;

        // --- STEP 1: RESOLVE ENCODINGS ---
        let x_enc = self.encoding.x.as_ref().ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self.encoding.y.as_ref().ok_or(ChartonError::Encoding("Y missing".into()))?;
        
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // --- STEP 2: GROUPING BY COLOR ---
        // Your transform logic ensures that if color encoding is used, the DF 
        // contains that column. We partition the DF to draw each group with its color.
        let group_column = context.spec.aesthetics.color.as_ref().map(|c| c.field.as_str());
        let groups = match group_column {
            Some(col_name) => df_source.partition_by([col_name], true)?,
            None => vec![df_source.clone()],
        };

        // Calculate the physical bar width. 
        // Note: transform_histogram_data provides 'bins' in x_enc which we use here.
        let bar_width = self.calculate_hist_bar_size(context)?;

        // --- STEP 3: RENDER GROUPS ---
        for group_df in groups {
            // Determine the group's color using the same logic as your Area renderer.
            let group_color = self.resolve_group_color(&group_df, context, &mark_config.color)?;

            let x_series = group_df.column(&x_enc.field)?.as_materialized_series();
            let y_series = group_df.column(&y_enc.field)?.as_materialized_series();

            // Transform data values into normalized [0, 1] space.
            let x_norms = x_scale.scale_type().normalize_series(x_scale, x_series)?;
            let y_norms = y_scale.scale_type().normalize_series(y_scale, y_series)?;
            
            // Baseline is always 0.0 in normalized space for frequency histograms.
            let y_baseline_norm = 0.0;

            for (opt_x, opt_y) in x_norms.into_iter().zip(y_norms.into_iter()) {
                let x_n = opt_x.unwrap_or(0.0);
                let y_n = opt_y.unwrap_or(0.0);

                // Convert normalized coordinates to screen pixels.
                // context.transform handles coordinate flipping and padding automatically.
                let (px, py_top) = context.transform(x_n, y_n);
                let (_, py_bottom) = context.transform(x_n, y_baseline_norm);

                let rect_height = (py_bottom - py_top).abs();

                backend.draw_rect(RectConfig {
                    // Bars are centered horizontally at the bin middle's pixel position.
                    x: (px - bar_width / 2.0) as Precision,
                    // min() ensures we handle the top of the bar correctly regardless of coordinate direction.
                    y: py_top.min(py_bottom) as Precision,
                    width: bar_width as Precision,
                    height: rect_height as Precision,
                    fill: group_color.clone(),
                    stroke: mark_config.stroke.clone(),
                    stroke_width: mark_config.stroke_width as Precision,
                    opacity: mark_config.opacity as Precision,
                });
            }
        }

        Ok(())
    }
}

// --- HELPER METHODS ---

impl Chart<MarkHist> {
    /// Calculates the bar width by measuring the raw data span (pre-expansion).
    /// 
    /// This is the most accurate approach because it:
    /// 1. Uses the actual data min/max to define the "true" data unit.
    /// 2. Uses the resolved 'bins' from encoding to divide that unit.
    /// 3. Normalizes this data-space width through the scale to account for any offsets.
    fn calculate_hist_bar_size(&self, context: &PanelContext) -> Result<f64, ChartonError> {
        // 1. Get the pre-resolved bin count.
        let n_bins = self.encoding.x.as_ref()
            .and_then(|x| x.bins)
            .ok_or_else(|| ChartonError::Encoding("Bin count not resolved".into()))? as f64;

        let x_field = &self.encoding.x.as_ref().unwrap().field;
        let x_scale = context.coord.get_x_scale();

        // 2. Access the transformed DataFrame to get the RAW data boundaries.
        let s = self.data.column(x_field)?;

        // 3. Find the actual data min and max (the bin centers' extent).
        let v_min = s.min::<f64>()?.ok_or(ChartonError::Data("X column is empty".into()))?;
        let v_max = s.max::<f64>()?.ok_or(ChartonError::Data("X column is empty".into()))?;
        
        // 4. Calculate the true data-space step between bins.
        // If there are N unique bins, the distance from the first center to the last 
        // center represents (N - 1) full bin widths.
        let data_step = if n_bins > 1.0 {
            (v_max - v_min) / (n_bins - 1.0)
        } else {
            // Fallback: if only one bin, we can't measure a step. 
            // We use a default fraction of the scale's domain.
            let (d0, d1) = x_scale.domain();
            (d1 - d0) * 0.5
        };

        // 5. Convert this data-space width into normalized [0, 1] distance.
        // We must use subtraction of two points to cancel out the Scale's internal expansion/offsets.
        let norm0 = x_scale.normalize(v_min);
        let norm1 = x_scale.normalize(v_min + data_step);

        // 6. Map to physical pixels.
        let (p0, _) = context.transform(norm0, 0.0);
        let (p1, _) = context.transform(norm1, 0.0);

        // 7. Calculate final width with a 0.95 gap factor.
        let theoretical_width = (p1 - p0).abs();
        
        Ok(theoretical_width * 0.95)
    }

    /// Resolves a single fill color for the entire histogram group.
    /// 
    /// This method is used when data is partitioned by a color aesthetic. 
    /// It ensures visual consistency by:
    /// 1. Identifying the data column associated with the color mapping.
    /// 2. Extracting the first value of that group (since all members of a group 
    ///    share the same categorical color).
    /// 3. Normalizing that value and mapping it to a specific `SingleColor` 
    ///    using the scale's palette or gradient mapper.
    /// 
    /// If no color encoding is provided, it returns the provided `fallback` color.
    fn resolve_group_color(
        &self, 
        df: &DataFrame, 
        context: &PanelContext, 
        fallback: &SingleColor
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            // Get the column mapped to the color aesthetic
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            
            // Map the first value of the group to a color to represent the whole series.
            // We use .head(Some(1)) to efficiently grab the representative value.
            let first_val_norm = s_trait.scale_type().normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = first_val_norm.get(0).unwrap_or(0.0);
            
            // Perform the final mapping from normalized value to a physical color.
            Ok(s_trait.mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| fallback.clone()))
        } else {
            // No color encoding: Use the static color defined in the Mark configuration.
            Ok(fallback.clone())
        }
    }
}