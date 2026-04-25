use crate::Precision;
use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{
    CircleConfig, MarkRenderer, PointElementConfig, PolygonConfig, RectConfig, RenderBackend,
};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::point::MarkPoint;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (High-Performance Parallel Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkPoint> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df = &self.data;
        let row_count = df.height();
        if row_count == 0 {
            return Ok(());
        }

        // --- 1. RESOLVE CONFIG & ENCODINGS ---
        // Replace the placeholders with actual ChartonError variants
        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("Point config missing".into()))?;
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X encoding missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y encoding missing".into()))?;

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // --- 2. VECTORIZED NORMALIZATION ---
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, df.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, df.column(&y_enc.field)?);

        // Resolve color mapping if it exists in the global aesthetics
        let color_norms = context.spec.aesthetics.color.as_ref().map(|m| {
            let s = m.scale_impl.as_ref();
            s.scale_type()
                .normalize_column(s, df.column(&m.field).unwrap())
        });

        // --- 3. METADATA HANDLING (SAFE FALLBACK) ---
        // We use .ok() to convert Result to Option so the renderer doesn't
        // crash if transform_point_data wasn't called (e.g., when color is missing).
        let sub_idx_col = df.column(&format!("{}_sub_idx", TEMP_SUFFIX)).ok();
        let groups_cnt_col = df.column(&format!("{}_groups_count", TEMP_SUFFIX)).ok();
        let swarm_rank_col = df.column(&format!("{}_swarm_rank", TEMP_SUFFIX)).ok();

        let unit_step_norm = (x_scale.normalize(1.0) - x_scale.normalize(0.0)).abs();
        let spread_factor = mark_config.size * unit_step_norm;

        // --- 4. GEOMETRY GENERATION ---
        let point_elements: Vec<_> = (0..row_count)
            .maybe_into_par_iter()
            .filter_map(|i| {
                let x_base = x_norms[i]?;
                let y_n = y_norms[i]?;

                // Graceful defaults for missing layout columns
                let sub_idx = sub_idx_col
                    .as_ref()
                    .and_then(|c| c.get_f64(i))
                    .unwrap_or(0.0);
                let n_groups = groups_cnt_col
                    .as_ref()
                    .and_then(|c| c.get_f64(i))
                    .unwrap_or(1.0);
                let swarm_rank = swarm_rank_col
                    .as_ref()
                    .and_then(|c| c.get_f64(i))
                    .unwrap_or(0.0);

                // --- 4.1 DODGE OFFSET ---
                let dodge_offset = if n_groups > 1.0 {
                    let actual_width =
                        mark_config.span / (n_groups + (n_groups - 1.0) * mark_config.spacing);
                    let width_norm = actual_width.min(mark_config.width) * unit_step_norm;
                    let spacing_norm = width_norm * mark_config.spacing;
                    (sub_idx - (n_groups - 1.0) / 2.0) * (width_norm + spacing_norm)
                } else {
                    0.0
                };

                // --- 4.2 SWARM OFFSET ---
                let swarm_offset = swarm_rank * spread_factor * 0.8;

                let final_x_n = x_base + dodge_offset + swarm_offset;
                let (px, py) = context.coord.transform(final_x_n, y_n, &context.panel);

                let fill = if let Some(ref norms) = color_norms {
                    self.resolve_color_from_value(norms[i], context, &mark_config.color)
                } else {
                    mark_config.color
                };

                Some((px, py, fill))
            })
            .collect();

        // --- 5. FINAL DRAWING ---
        for (x, y, color) in point_elements {
            backend.draw_circle(CircleConfig {
                x: x as Precision,
                y: y as Precision,
                radius: (mark_config.size / 2.0) as Precision,
                fill: color,
                stroke: mark_config.stroke,
                stroke_width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
            });
        }

        Ok(())
    }
}

// ============================================================================
// HELPER METHODS & GEOMETRY DISPATCH
// ============================================================================

impl Chart<MarkPoint> {
    /// Maps a normalized value to a color using the registered scale mapper.
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

    /// Maps a normalized value to a point size.
    fn resolve_size_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: f64,
    ) -> f64 {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.size) {
            mapping
                .scale_impl
                .mapper()
                .as_ref()
                .map(|m| m.map_to_size(v))
                .unwrap_or(fallback)
        } else {
            fallback
        }
    }

    /// Maps a normalized value to a specific PointShape.
    fn resolve_shape_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: PointShape,
    ) -> PointShape {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.shape) {
            let s_trait = mapping.scale_impl.as_ref();
            mapping
                .scale_impl
                .mapper()
                .as_ref()
                .map(|m| m.map_to_shape(v, s_trait.logical_max()))
                .unwrap_or(fallback)
        } else {
            fallback
        }
    }

    /// Dispatches the appropriate backend draw call for the given PointShape.
    fn emit_draw_call(&self, backend: &mut dyn RenderBackend, config: PointElementConfig) {
        let PointElementConfig {
            x,
            y,
            shape,
            size,
            fill,
            stroke,
            stroke_width,
            opacity,
        } = config;

        match shape {
            PointShape::Circle => {
                backend.draw_circle(CircleConfig {
                    x: x as Precision,
                    y: y as Precision,
                    radius: size as Precision,
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    opacity: opacity as Precision,
                });
            }
            PointShape::Square => {
                let side = size * 2.0;
                backend.draw_rect(RectConfig {
                    x: (x - size) as Precision,
                    y: (y - size) as Precision,
                    width: side as Precision,
                    height: side as Precision,
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    opacity: opacity as Precision,
                });
            }
            _ => {
                let (sides, rotation, scale_adj) = match shape {
                    PointShape::Diamond => (4, 0.0, 1.2),
                    PointShape::Triangle => (3, -std::f64::consts::FRAC_PI_2, 1.1),
                    PointShape::Pentagon => (5, -std::f64::consts::FRAC_PI_2, 1.0),
                    PointShape::Hexagon => (6, 0.0, 1.0),
                    PointShape::Octagon => (8, std::f64::consts::FRAC_PI_8, 1.0),
                    _ => (0, 0.0, 0.0),
                };

                let points = if shape == PointShape::Star {
                    self.calculate_star(x, y, size * 1.2, size * 0.5, 5)
                } else {
                    self.calculate_polygon(x, y, size * scale_adj, sides, rotation)
                };

                backend.draw_polygon(PolygonConfig {
                    points: points
                        .iter()
                        .map(|p| (p.0 as Precision, p.1 as Precision))
                        .collect(),
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    fill_opacity: opacity as Precision,
                    stroke_opacity: 1.0,
                });
            }
        }
    }

    fn calculate_polygon(
        &self,
        cx: f64,
        cy: f64,
        r: f64,
        sides: usize,
        rot: f64,
    ) -> Vec<(f64, f64)> {
        (0..sides)
            .map(|i| {
                let angle = rot + 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
                (cx + r * angle.cos(), cy + r * angle.sin())
            })
            .collect()
    }

    fn calculate_star(
        &self,
        cx: f64,
        cy: f64,
        out_r: f64,
        in_r: f64,
        pts: usize,
    ) -> Vec<(f64, f64)> {
        (0..(pts * 2))
            .map(|i| {
                let angle =
                    -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * (i as f64) / (pts as f64);
                let r = if i % 2 == 0 { out_r } else { in_r };
                (cx + r * angle.cos(), cy + r * angle.sin())
            })
            .collect()
    }
}
