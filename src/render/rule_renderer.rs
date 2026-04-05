use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{LineConfig, MarkRenderer, RenderBackend};
use crate::error::ChartonError;
use crate::mark::rule::MarkRule;
use crate::visual::color::SingleColor;

// ============================================================================
// MARK RENDERING (Rule Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkRule> {
    /// Renders rule marks (straight lines spanning a specific range).
    /// Typically used for axis-parallel lines or range indicators.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let ds = &self.data;
        if ds.row_count == 0 {
            return Ok(());
        }

        // --- STEP 1: VALIDATION ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X is missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y is missing".into()))?;

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkRule configuration is missing".to_string()))?;

        // --- STEP 2: POSITION NORMALIZATION (Vectorized) ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // Normalize X (The constant coordinate for vertical rules)
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);

        // Normalize Y and Y2 (The range coordinates)
        let y1_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_enc.field)?);

        let y2_norms = if let Some(ref y2_enc) = self.encoding.y2 {
            // Case A: User provided an explicit end-point column
            Some(
                y_scale
                    .scale_type()
                    .normalize_column(y_scale, ds.column(&y2_enc.field)?),
            )
        } else {
            // Case B: No y2 provided, the rule spans the full logical height [0.0, 1.0]
            None
        };

        // --- STEP 3: COLOR RESOLUTION ---
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

        // --- STEP 4: GEOMETRY PROJECTION & RENDERING ---
        let stroke_width = mark_config.stroke_width as Precision;
        let is_flipped = context.coord.is_flipped();

        for i in 0..ds.row_count {
            // Skip if the base coordinates are missing
            let (Some(xn), Some(yn1)) = (x_norms[i], y1_norms[i]) else {
                continue;
            };

            // Determine yn2: use mapped value or default to full axis (1.0)
            let yn2 = y2_norms.as_ref().and_then(|norms| norms[i]).unwrap_or(1.0);

            // Project endpoints to physical pixels
            let (p1_x, p1_y) = context.coord.transform(xn, yn1, &context.panel);
            let (p2_x, p2_y) = context.coord.transform(xn, yn2, &context.panel);

            // Correct orientation based on coord_flip logic:
            // Standard: Constant X, Y ranges from y1 to y2 (Vertical)
            // Flipped: Constant Y, X ranges from x1 to x2 (Horizontal)
            let (final_x1, final_y1, final_x2, final_y2) = if !is_flipped {
                (p1_x, p1_y, p1_x, p2_y)
            } else {
                (p1_x, p1_y, p2_x, p1_y)
            };

            // Resolve color
            let line_color = if let Some(ref norms) = color_norms {
                self.resolve_color_from_value(norms[i], context, &mark_config.color)
            } else {
                mark_config.color
            };

            backend.draw_line(LineConfig {
                x1: final_x1 as Precision,
                y1: final_y1 as Precision,
                x2: final_x2 as Precision,
                y2: final_y2 as Precision,
                color: line_color,
                width: stroke_width,
                opacity: mark_config.opacity as Precision,
                dash: vec![], // Rules are typically solid; add dash support if needed in config
            });
        }

        Ok(())
    }
}

impl Chart<MarkRule> {
    /// Reusable aesthetic color resolver.
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
