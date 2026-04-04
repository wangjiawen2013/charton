use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{
    CircleConfig, MarkRenderer, PointElementConfig, PolygonConfig, RectConfig, RenderBackend,
};
use crate::error::ChartonError;
use crate::mark::point::MarkPoint;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (High-Performance Parallel Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkPoint> {
    /// Orchestrates the transformation of raw data into visual geometries using a
    /// parallelized pipeline for maximum throughput.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        let row_count = df_source.height();

        // Early exit for empty datasets to skip allocation overhead.
        if row_count == 0 {
            return Ok(());
        }

        // --- STEP 1: SPECIFICATION VALIDATION ---
        // Ensure all required encodings and configurations exist before starting heavy computation.
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding is missing".to_string()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y-axis encoding is missing".to_string()))?;
        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkPoint configuration is missing".to_string()))?;

        // --- STEP 2: POSITION NORMALIZATION (Vectorized via Polars) ---
        // Extract columns and normalize data to [0, 1] range using scale-specific logic.
        let x_series = df_source.column(&x_enc.field)?;
        let y_series = df_source.column(&y_enc.field)?;

        let x_scale_trait = context.coord.get_x_scale();
        let y_scale_trait = context.coord.get_y_scale();

        let x_norms = x_scale_trait
            .scale_type()
            .normalize_column(x_scale_trait, &x_series);
        let y_norms = y_scale_trait
            .scale_type()
            .normalize_column(y_scale_trait, &y_series);

        // --- STEP 3: PARALLEL AESTHETIC RESOLUTION ---
        // We pre-calculate all visual properties (Color, Shape, Size) in parallel
        // to avoid expensive branching inside the drawing loop.

        // Resolve Color: Data-driven mapping vs. Static fallback.
        let color_vec: Vec<SingleColor> = if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            let l_max = s_trait.logical_max();
            let mapper = s_trait.mapper();
            let s_norms = s_trait.scale_type().normalize_column(s_trait, &s);

            s_norms
                .into_par_iter()
                .map(|opt_n| {
                    mapper
                        .as_ref()
                        .map(|m| m.map_to_color(opt_n.unwrap_or(0.0), l_max))
                        .unwrap_or_else(|| SingleColor::from("#333333"))
                })
                .collect()
        } else {
            vec![mark_config.color; row_count]
        };

        // Resolve Shape: Data-driven mapping vs. Static fallback.
        let shape_vec: Vec<PointShape> = if let Some(ref mapping) = context.spec.aesthetics.shape {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            let l_max = s_trait.logical_max();
            let mapper = s_trait.mapper();
            let s_norms = s_trait.scale_type().normalize_column(s_trait, &s);

            s_norms
                .into_par_iter()
                .map(|opt_n| {
                    mapper
                        .as_ref()
                        .map(|m| m.map_to_shape(opt_n.unwrap_or(0.0), l_max))
                        .unwrap_or(PointShape::Circle)
                })
                .collect()
        } else {
            vec![mark_config.shape; row_count]
        };

        // Resolve Size: Data-driven mapping vs. Static fallback.
        let size_vec: Vec<f64> = if let Some(ref mapping) = context.spec.aesthetics.size {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            let mapper = s_trait.mapper();
            let s_norms = s_trait.scale_type().normalize_column(s_trait, &s);

            s_norms
                .into_par_iter()
                .map(|opt_n| {
                    mapper
                        .as_ref()
                        .map(|m| m.map_to_size(opt_n.unwrap_or(0.0)))
                        .unwrap_or(mark_config.size)
                })
                .collect()
        } else {
            vec![mark_config.size; row_count]
        };

        // --- STEP 4: COORDINATE PROJECTION (Parallel) ---
        // Transform normalized [0, 1] units into physical screen pixels (px, py).
        // This is a CPU-bound task that benefits significantly from Rayon.
        let render_configs: Vec<PointElementConfig> = (0..row_count)
            .into_par_iter()
            .map(|i| {
                let x_n = x_norms[i].unwrap_or(0.0);
                let y_n = y_norms[i].unwrap_or(0.0);
                let (px, py) = context.transform(x_n, y_n);

                PointElementConfig {
                    x: px,
                    y: py,
                    shape: shape_vec[i],
                    size: size_vec[i],
                    fill: color_vec[i],
                    stroke: mark_config.stroke,
                    stroke_width: mark_config.stroke_width,
                    opacity: mark_config.opacity,
                }
            })
            .collect();

        // --- STEP 5: SEQUENTIAL DRAW DISPATCH ---
        // Backends (SVG, Canvas, etc.) are usually single-threaded due to state management.
        // We iterate through our pre-computed configs and emit draw calls.
        for config in render_configs {
            self.emit_draw_call(backend, config);
        }

        Ok(())
    }
}

// ============================================================================
// GEOMETRY DISPATCH (Private helper)
// ============================================================================

impl Chart<MarkPoint> {
    /// Dispatches the appropriate backend draw call for the given PointShape.
    /// This handles the specific geometry math for each shape type.
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

        // Shared casting logic to match the backend's expected Precision (f32/f64).
        let common_opacity = opacity as Precision;
        let common_stroke_w = stroke_width as Precision;

        match shape {
            PointShape::Circle => {
                backend.draw_circle(CircleConfig {
                    x: x as Precision,
                    y: y as Precision,
                    radius: size as Precision,
                    fill,
                    stroke,
                    stroke_width: common_stroke_w,
                    opacity: common_opacity,
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
                    stroke_width: common_stroke_w,
                    opacity: common_opacity,
                });
            }
            // Polygon-based shapes (Diamond, Triangle, etc.) use a helper to calculate vertices.
            _ => {
                let (sides, rotation, scale_adj) = match shape {
                    PointShape::Diamond => (4, 0.0, 1.2),
                    PointShape::Triangle => (3, -std::f64::consts::FRAC_PI_2, 1.1),
                    PointShape::Pentagon => (5, -std::f64::consts::FRAC_PI_2, 1.0),
                    PointShape::Hexagon => (6, 0.0, 1.0),
                    PointShape::Octagon => (8, std::f64::consts::FRAC_PI_8, 1.0),
                    _ => (0, 0.0, 0.0), // Star is handled separately or as default.
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
                    stroke_width: common_stroke_w,
                    fill_opacity: common_opacity,
                    stroke_opacity: 1.0,
                });
            }
        }
    }

    /// Computes vertices for regular polygons.
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

    /// Computes vertices for star shapes.
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
