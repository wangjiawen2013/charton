use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{LineConfig, MarkRenderer, RenderBackend};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::rule::MarkRule;
use crate::visual::color::SingleColor;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (Rule Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkRule> {
    /// Renders "Rule" marks, which are straight line segments typically used for
    /// axis rulers, error bars, or range indicators.
    ///
    /// This implementation uses a row-independent parallel approach (similar to PointMark)
    /// because each rule is a standalone geometry that doesn't require connecting
    /// points or path interpolation.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        let row_count = df_source.height();

        if row_count == 0 {
            return Ok(());
        }

        // --- STEP 1: SPECIFICATION VALIDATION ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X encoding is missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y encoding is missing".into()))?;
        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkRule configuration is missing".into()))?;

        // --- STEP 2: POSITION & AESTHETIC NORMALIZATION ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // Vectorized normalization of coordinates
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, df_source.column(&x_enc.field)?);
        let y1_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, df_source.column(&y_enc.field)?);

        // Normalize Y2 if provided; defaults to 1.0 (top of the scale) for vertical rules
        let y2_norms = self.encoding.y2.as_ref().map(|e| {
            y_scale
                .scale_type()
                .normalize_column(y_scale, &df_source.column(&e.field).unwrap())
        });

        // Pre-normalize color aesthetics for data-driven mapping
        let color_norms = context.spec.aesthetics.color.as_ref().map(|m| {
            let s = m.scale_impl.as_ref();
            s.scale_type()
                .normalize_column(s, &df_source.column(&m.field).unwrap())
        });

        let is_flipped = context.coord.is_flipped();

        // --- STEP 3: PARALLEL GEOMETRY PROJECTION ---
        let render_configs: Vec<LineConfig> = (0..row_count)
            .maybe_into_par_iter()
            .filter_map(|i| {
                let x_n = x_norms[i]?;
                let yn1 = y1_norms[i]?;
                let yn2 = y2_norms.as_ref().and_then(|ns| ns[i]).unwrap_or(1.0);

                // Transform normalized coordinates [0, 1] to screen pixel space
                let (p1_x, p1_y) = context.coord.transform(x_n, yn1, &context.panel);
                let (p2_x, p2_y) = context.coord.transform(x_n, yn2, &context.panel);

                // Determine orientation based on coordinate system flipping:
                // Standard: Constant X, Y spans from y1 to y2 (Vertical Rule)
                // Flipped: Constant Y, X spans from x1 to x2 (Horizontal Rule)
                let (x1, y1, x2, y2) = if !is_flipped {
                    (p1_x, p1_y, p1_x, p2_y)
                } else {
                    (p1_x, p1_y, p2_x, p1_y)
                };

                // Resolve color: Priority is Data Mapping > Mark Config Fallback
                let final_color = self.resolve_color_from_value(
                    color_norms.as_ref().and_then(|n| n[i]),
                    context,
                    &mark_config.color,
                );

                Some(LineConfig {
                    x1: x1 as Precision,
                    y1: y1 as Precision,
                    x2: x2 as Precision,
                    y2: y2 as Precision,
                    color: final_color,
                    width: mark_config.stroke_width as Precision,
                    opacity: mark_config.opacity as Precision,
                    dash: vec![],
                })
            })
            .collect();

        // --- STEP 4: SEQUENTIAL DRAW DISPATCH ---
        // Lines are drawn in original data order to maintain deterministic Z-indexing.
        for config in render_configs {
            backend.draw_line(config);
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
