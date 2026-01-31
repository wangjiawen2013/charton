use crate::core::layer::{MarkRenderer, RenderBackend, LineConfig, CircleConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::Precision;
use crate::mark::errorbar::MarkErrorBar;
use crate::error::ChartonError;

/// Implementation of `MarkRenderer` for Error Bars.
/// 
/// This renderer handles the visualization of uncertainty intervals by drawing 
/// a central whisker and horizontal caps at the boundaries.
impl MarkRenderer for Chart<MarkErrorBar> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        
        // Early return if the dataframe is empty to optimize performance.
        if df_source.df.height() == 0 { return Ok(()); }

        // --- STEP 1: SPECIFICATION VALIDATION ---
        // Error bars require at least three encodings: X (position), 
        // Y (lower bound), and Y2 (upper bound).
        let x_enc = self.encoding.x.as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding is missing".to_string()))?;
        let y_enc = self.encoding.y.as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y-axis (min) encoding is missing".to_string()))?;
        let y2_enc = self.encoding.y2.as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y2-axis (max) encoding is missing".to_string()))?;
        
        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkErrorBar configuration is missing".to_string()))?;

        // --- STEP 2: POSITION NORMALIZATION ---
        // Extract raw data columns as Polars series.
        let x_series = df_source.column(&x_enc.field)?;
        let y_min_series = df_source.column(&y_enc.field)?;
        let y_max_series = df_source.column(&y2_enc.field)?;

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // Perform vectorized normalization to map data to [0.0, 1.0] range.
        let x_norms = x_scale.scale_type().normalize_series(x_scale, &x_series)?;
        let y_min_norms = y_scale.scale_type().normalize_series(y_scale, &y_min_series)?;
        let y_max_norms = y_scale.scale_type().normalize_series(y_scale, &y_max_series)?;

        // --- STEP 3: RENDERING LOOP ---
        let color = mark_config.color.clone();
        let width = mark_config.stroke_width as Precision;
        let cap_len = mark_config.cap_length as Precision;

        // Iterate through normalized data points and project them to physical space.
        for ((x_n, y_min_n), y_max_n) in x_norms.into_iter()
            .zip(y_min_norms.into_iter())
            .zip(y_max_norms.into_iter()) 
        {
            let x_norm = x_n.unwrap_or(0.0);
            let y_min_norm = y_min_n.unwrap_or(0.0);
            let y_max_norm = y_max_n.unwrap_or(0.0);

            // `context.transform` automatically handles coordinate flipping (coord_flip) 
            // and panel margins, converting [0,1] units to pixel coordinates.
            let (x_pix, y_min_pix) = context.transform(x_norm, y_min_norm);
            let (_unused, y_max_pix) = context.transform(x_norm, y_max_norm);
            
            let x = x_pix as Precision;
            let y_min = y_min_pix as Precision;
            let y_max = y_max_pix as Precision;

            // 1. Draw the main vertical whisker.
            backend.draw_line(LineConfig {
                x1: x, y1: y_min,
                x2: x, y2: y_max,
                color: color.clone(),
                width,
            });

            // 2. Draw horizontal Caps (Top and Bottom boundaries).
            // Bottom Cap: horizontal segment at the lower bound.
            backend.draw_line(LineConfig {
                x1: x - cap_len, y1: y_min,
                x2: x + cap_len, y2: y_min,
                color: color.clone(),
                width,
            });
            // Top Cap: horizontal segment at the upper bound.
            backend.draw_line(LineConfig {
                x1: x - cap_len, y1: y_max,
                x2: x + cap_len, y2: y_max,
                color: color.clone(),
                width,
            });

            // 3. Draw Center Point (Optional).
            // Usually represents the mean or median of the distribution.
            if mark_config.show_center {
                let y_mean_norm = (y_min_norm + y_max_norm) / 2.0;
                let (_, y_mean_pix) = context.transform(x_norm, y_mean_norm);
                
                backend.draw_circle(CircleConfig {
                    x,
                    y: y_mean_pix as Precision,
                    radius: 3.0 as Precision,
                    fill: color.clone(),
                    stroke: color.clone(),
                    stroke_width: 0.0 as Precision,
                    opacity: mark_config.opacity as Precision, // Circle keeps opacity for better aesthetics
                });
            }
        }

        Ok(())
    }
}