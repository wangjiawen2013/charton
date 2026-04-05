use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RectConfig, RenderBackend};
use crate::error::ChartonError;
use crate::mark::tick::MarkTick;
use crate::visual::color::SingleColor;

// ============================================================================
// MARK RENDERING (Tick Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkTick> {
    /// Renders tick marks by transforming data points into thin rectangular geometries.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let ds = &self.data;

        // Early return if no data to process.
        if ds.row_count == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkTick configuration is missing".to_string()))?;

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

        // --- STEP 2: POSITION NORMALIZATION (Vectorized) ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // Perform vectorized normalization directly on internal ColumnVectors.
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_enc.field)?);

        // --- STEP 3: COLOR RESOLUTION ---
        // Pre-normalize the color column if a mapping exists.
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

        // --- STEP 4: GEOMETRY PROJECTION & EMIT ---
        let thickness = mark_config.thickness;
        let band_size = mark_config.band_size;
        let opacity = mark_config.opacity;
        let is_flipped = context.coord.is_flipped();

        for i in 0..ds.row_count {
            // Skip points where X or Y are null to avoid rendering at (0,0).
            let (Some(xn), Some(yn)) = (x_norms[i], y_norms[i]) else {
                continue;
            };

            // 4.1 Coordinate Projection: [0, 1] -> Pixels
            let (px, py) = context.coord.transform(xn, yn, &context.panel);

            // 4.2 Color Resolution: Resolve mapping or use static fallback
            let fill_color = if let Some(ref norms) = color_norms {
                self.resolve_color_from_value(norms[i], context, &mark_config.color)
            } else {
                mark_config.color
            };

            // 4.3 Tick Geometry Calculation:
            // Ticks are centered on the (px, py) coordinate.
            let (rect_x, rect_y, rect_w, rect_h) = if !is_flipped {
                // Vertical ticks: narrow width, tall height
                (
                    px - thickness / 2.0,
                    py - band_size / 2.0,
                    thickness,
                    band_size,
                )
            } else {
                // Horizontal ticks: wide width, narrow height
                (
                    px - band_size / 2.0,
                    py - thickness / 2.0,
                    band_size,
                    thickness,
                )
            };

            backend.draw_rect(RectConfig {
                x: rect_x as Precision,
                y: rect_y as Precision,
                width: rect_w as Precision,
                height: rect_h as Precision,
                fill: fill_color,
                stroke: mark_config.color, // Border matches mark color
                stroke_width: 0.0,         // Ticks are typically filled only
                opacity: opacity as Precision,
            });
        }

        Ok(())
    }
}

impl Chart<MarkTick> {
    /// Shared utility to map a normalized data value to its aesthetic color.
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
