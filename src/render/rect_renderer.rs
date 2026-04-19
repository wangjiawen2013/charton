use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RectConfig, RenderBackend};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::rect::MarkRect;
use crate::visual::color::SingleColor;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (Rect/Heatmap Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkRect> {
    /// Orchestrates the transformation of aggregated data into rectangular geometries.
    ///
    /// Optimized for Heatmaps and Binned 2D plots by:
    /// 1. Removing redundant grouping for improved parallel throughput.
    /// 2. Using a unified 'Calculate -> Collect -> Emit' pipeline.
    /// 3. Ensuring deterministic Z-indexing via sequential backend dispatch.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        let row_count = df_source.height();

        // Guard against empty datasets to prevent unnecessary allocation
        if row_count == 0 {
            return Ok(());
        }

        // --- STEP 1: SPECIFICATION VALIDATION ---
        // Ensure required encodings and mark configurations exist
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
        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkRect configuration is missing".into()))?;

        // --- STEP 2: POSITION & AESTHETIC NORMALIZATION ---
        // Vectorized normalization: maps raw data values to a [0, 1] logical space
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, df_source.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, df_source.column(&y_enc.field)?);

        // Pre-normalize color aesthetics (Essential for continuous Heatmap gradients)
        let color_norms = context.spec.aesthetics.color.as_ref().map(|m| {
            let s = m.scale_impl.as_ref();
            s.scale_type()
                .normalize_column(s, df_source.column(&m.field).unwrap())
        });

        // --- STEP 3: SIZE CALCULATION ---
        // Determine the fixed pixel dimensions of a single tile based on bin configuration
        let (rect_width, rect_height) = self.calculate_rect_size(context);

        // --- STEP 4: PARALLEL GEOMETRY GENERATION ---
        // Map normalized data to screen-space RectConfig objects
        let render_configs: Vec<RectConfig> = (0..row_count)
            .maybe_into_par_iter()
            .filter_map(|i| {
                let x_n = x_norms[i]?;
                let y_n = y_norms[i]?;

                // 1. Coordinate Transformation: Get the center point of the rectangle
                let (px, py) = context.coord.transform(x_n, y_n, &context.panel);

                // 2. Aesthetic Resolution: Resolve fill color from scale or fallback
                let fill = self.resolve_color_from_value(
                    color_norms.as_ref().and_then(|n| n[i]),
                    context,
                    &mark_config.color,
                );

                // 3. Rect Boundary Calculation: Offset from center to top-left corner
                Some(RectConfig {
                    x: (px - rect_width / 2.0) as Precision,
                    y: (py - rect_height / 2.0) as Precision,
                    width: rect_width as Precision,
                    height: rect_height as Precision,
                    fill,
                    stroke: mark_config.stroke,
                    stroke_width: mark_config.stroke_width as Precision,
                    opacity: mark_config.opacity as Precision,
                })
            })
            .collect();

        // --- STEP 5: SEQUENTIAL DRAW DISPATCH ---
        // Final rendering pass to the backend. Sequential execution ensures
        // that the drawing order matches the data order (stable Z-indexing).
        for config in render_configs {
            backend.draw_rect(config);
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
