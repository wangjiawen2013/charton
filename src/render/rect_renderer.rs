use crate::core::layer::{MarkRenderer, RenderBackend, RectConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::mark::rect::MarkRect;
use crate::error::ChartonError;
use crate::Precision;

impl MarkRenderer for Chart<MarkRect> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df = &self.data.df;
        if df.height() == 0 { return Ok(()); }

        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkRect configuration is missing".into()))?;

        // --- STEP 1: POSITIONING ---
        let x_enc = self.encoding.x.as_ref().ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self.encoding.y.as_ref().ok_or(ChartonError::Encoding("Y missing".into()))?;
        
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        let x_series = df.column(&x_enc.field)?.as_materialized_series();
        let y_series = df.column(&y_enc.field)?.as_materialized_series();

        // Standardize data to [0.0, 1.0] normalized space
        let x_norms = x_scale.scale_type().normalize_series(x_scale, x_series)?;
        let y_norms = y_scale.scale_type().normalize_series(y_scale, y_series)?;

        // --- STEP 2: SIZE CALCULATION ---
        // We now calculate sizes based on the pre-resolved 'bins' count.
        // This ensures the rectangles fill the exact width/height allocated by the scale.
        let (rect_width, rect_height) = self.calculate_rect_size(context);

        // --- STEP 3: COLOR MAPPING ---
        let color_iter = self.resolve_rect_colors(df, context, &mark_config.color)?;

        // --- STEP 4: RENDERING LOOP ---
        // Iterate through normalized coordinates and draw centered rectangles
        for ((opt_x, opt_y), fill_color) in x_norms.into_iter()
            .zip(y_norms.into_iter())
            .zip(color_iter) 
        {
            let x_n = opt_x.unwrap_or(0.0);
            let y_n = opt_y.unwrap_or(0.0);

            // Convert normalized [0,1] to pixel coordinates
            let (px, py) = context.transform(x_n, y_n);

            backend.draw_rect(RectConfig {
                // px/py are centers; we offset by half-width/height to get the top-left corner
                x: (px - rect_width / 2.0) as Precision,
                y: (py - rect_height / 2.0) as Precision,
                width: rect_width as Precision,
                height: rect_height as Precision,
                fill: fill_color,
                stroke: mark_config.stroke.clone(),
                stroke_width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
            });
        }

        Ok(())
    }
}
