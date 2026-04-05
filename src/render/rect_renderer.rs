use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RectConfig, RenderBackend};
use crate::error::ChartonError;
use crate::mark::rect::MarkRect;
use crate::visual::color::SingleColor;

// ============================================================================
// MARK RENDERING (Rect/Heatmap Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkRect> {
    /// Renders rectangles, typically used for heatmaps or binned 2D plots.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let ds = &self.data;
        if ds.row_count == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkRect configuration is missing".into()))?;

        // --- STEP 1: ENCODING VALIDATION ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding is missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y-axis encoding is missing".into()))?;

        // --- STEP 2: VECTORIZED NORMALIZATION ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // Standardize data to [0.0, 1.0] normalized space using pre-computed columns
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_enc.field)?);

        // --- STEP 3: SIZE CALCULATION ---
        // Rectangles in heatmaps usually fill a specific bin width/height.
        let (rect_width, rect_height) = self.calculate_rect_size(context);

        // --- STEP 4: COLOR RESOLUTION ---
        let color_norms = if let Some(ref mapping) = context.spec.aesthetics.color {
            let s_trait = mapping.scale_impl.as_ref();
            Some(
                s_trait
                    .scale_type()
                    .normalize_column(s_trait, ds.column(&mapping.field)?),
            )
        } else {
            None
        };

        // --- STEP 5: RENDERING LOOP ---
        for i in 0..ds.row_count {
            // Skip rows with missing coordinates
            let (Some(xn), Some(yn)) = (x_norms[i], y_norms[i]) else {
                continue;
            };

            // Convert normalized [0,1] to pixel coordinates (usually the center of the bin)
            let (px, py) = context.coord.transform(xn, yn, &context.panel);

            // Resolve color for this specific tile
            let fill_color = if let Some(ref norms) = color_norms {
                self.resolve_color_from_value(norms[i], context, &mark_config.color)
            } else {
                mark_config.color
            };

            backend.draw_rect(RectConfig {
                // px/py represent the center; we offset by half-dimensions to get top-left
                x: (px - rect_width / 2.0) as Precision,
                y: (py - rect_height / 2.0) as Precision,
                width: rect_width as Precision,
                height: rect_height as Precision,
                fill: fill_color,
                stroke: mark_config.stroke,
                stroke_width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
            });
        }

        Ok(())
    }
}

impl Chart<MarkRect> {
    /// Calculates the pixel dimensions for a single rectangle tile based on bin counts.
    fn calculate_rect_size(&self, context: &PanelContext) -> (f64, f64) {
        let x_bins = self.encoding.x.as_ref().and_then(|e| e.bins).unwrap_or(1);
        let y_bins = self.encoding.y.as_ref().and_then(|e| e.bins).unwrap_or(1);

        // Logical step in normalized [0.0, 1.0] space
        let x_step = 1.0 / (x_bins as f64);
        let y_step = 1.0 / (y_bins as f64);

        // Transform logical delta into pixel delta
        let (p0_x, p0_y) = context.coord.transform(0.0, 0.0, &context.panel);
        let (p1_x, p1_y) = context.coord.transform(x_step, y_step, &context.panel);

        ((p1_x - p0_x).abs(), (p1_y - p0_y).abs())
    }

    /// Resolves color mapping for a normalized value.
    fn resolve_color_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> SingleColor {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.color) {
            let s_trait = mapping.scale_impl.as_ref();
            s_trait
                .mapper()
                .as_ref()
                .map(|m| m.map_to_color(v, s_trait.logical_max()))
                .unwrap_or(*fallback)
        } else {
            *fallback
        }
    }
}
