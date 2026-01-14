use crate::core::layer::{MarkRenderer, LegendRenderer, RenderBackend};
use crate::core::context::SharedRenderingContext;
use crate::chart::Chart;
use crate::mark::point::MarkPoint;
use crate::scale::Scale;
use crate::error::ChartonError;
use crate::visual::shape::PointShape;
use itertools::izip;

// ============================================================================
// MARK RENDERING (The main data-to-geometry loop)
// ============================================================================

impl MarkRenderer for Chart<MarkPoint> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        if df_source.df.height() == 0 { return Ok(()); }

        let x_enc = self.encoding.x.as_ref().unwrap();
        let y_enc = self.encoding.y.as_ref().unwrap();
        let mark_config = self.mark.as_ref().unwrap();

        // --- 1. COORDINATE NORMALIZATION ---
        // Vectorized normalization: Processes the entire column at once for speed.
        let x_series = df_source.column(&x_enc.field)?;
        let y_series = df_source.column(&y_enc.field)?;

        let x_norms = x_enc.scale.as_ref().unwrap_or(&Scale::Linear)
            .normalize_series(context.coord.get_x_scale(), &x_series)?;
        
        let y_norms = y_enc.scale.as_ref().unwrap_or(&Scale::Linear)
            .normalize_series(context.coord.get_y_scale(), &y_series)?;

        // --- 2. COLOR AESTHETIC ITERATOR ---
        let color_iter: Box<dyn Iterator<Item = String>> = if let (Some(enc), Some((scale_trait, mapper))) = (&self.encoding.color, &context.aesthetics.color) {
            let s = df_source.column(&enc.field)?;
            let scale_type = enc.scale.as_ref().unwrap_or(&Scale::Discrete);
            let logical_max = scale_trait.logical_max();

            let norms = scale_type.normalize_series(scale_trait.as_ref(), &s)?;
            
            // We clone the mapper to move it into the closure.
            // We use into_iter() on the ChunkedArray, but we need to ensure the 
            // iterator is "Static" or "Owned". 
            // The easiest way to resolve the lifetime is to map it to a Vec first,
            // or use a capturing closure that takes ownership of the 'norms'.
            let mapper = mapper.clone();
            let color_vec: Vec<String> = norms.into_iter().map(move |opt_n| {
                mapper.map_to_color(opt_n.unwrap_or(0.0), logical_max)
            }).collect();

            Box::new(color_vec.into_iter())
        } else {
            let default_c = mark_config.color.as_ref().map(|c| c.get_color()).unwrap_or_else(|| "black".into());
            Box::new(std::iter::repeat(default_c))
        };

        // --- 3. SIZE AESTHETIC ITERATOR ---
        let size_iter: Box<dyn Iterator<Item = f64>> = if let (Some(enc), Some((scale_trait, mapper))) = (&self.encoding.size, &context.aesthetics.size) {
            let s = df_source.column(&enc.field)?;
            let scale_type = &enc.scale; 

            let norms = scale_type.normalize_series(scale_trait.as_ref(), &s)?;
            
            // 1. Clone the mapper so it can be moved into the closure
            let mapper = mapper.clone();

            // 2. Collect into a Vec to break the dependency on the 'norms' lifetime
            let size_vec: Vec<f64> = norms.into_iter().map(move |opt_n| {
                mapper.map_to_size(opt_n.unwrap_or(0.0))
            }).collect();

            // 3. Return an iterator over the owned Vec
            Box::new(size_vec.into_iter())
        } else {
            // Constant fallback
            Box::new(std::iter::repeat(mark_config.size))
        };

        // --- 4. MASTER RENDER LOOP ---
        // Merges all streams and converts normalized coordinates [0, 1] to screen pixels.
        for (x_n, y_n, fill_color, size) in izip!(x_norms.into_iter(), y_norms.into_iter(), color_iter, size_iter) {
            let x_norm = x_n.unwrap_or(0.0);
            let y_norm = y_n.unwrap_or(0.0);
            
            let (px, py) = context.coord.transform(x_norm, y_norm, &context.panel);

            self.emit_draw_call(
                backend,
                &mark_config.shape,
                px, py,
                size,
                &fill_color,
                &mark_config.stroke.as_ref().map(|c| c.get_color()).unwrap_or_else(|| "none".into()),
                mark_config.stroke_width,
                mark_config.opacity
            );
        }

        Ok(())
    }
}

// ============================================================================
// GEOMETRY DISPATCH (Private helper)
// ============================================================================

impl Chart<MarkPoint> {
    /// Dispatches drawing commands to the backend by calculating geometric primitives.
    /// All shapes are centered at the provided (px, py) coordinates.
    fn emit_draw_call(
        &self,
        backend: &mut dyn RenderBackend,
        shape: &PointShape,
        px: f64,
        py: f64,
        size: f64,
        fill: &str,
        stroke: &str,
        stroke_width: f64,
        opacity: f64,
    ) {
        let fill_opt = if fill == "none" { None } else { Some(fill) };
        let stroke_opt = if stroke == "none" { None } else { Some(stroke) };

        match shape {
            PointShape::Circle => {
                backend.draw_circle(px, py, size, fill_opt, stroke_opt, stroke_width, opacity);
            }
            PointShape::Square => {
                let side = size * 2.0;
                backend.draw_rect(px - size, py - size, side, side, fill_opt, stroke_opt, stroke_width, opacity);
            }
            PointShape::Diamond => {
                let points = self.calculate_polygon(px, py, size * 1.2, 4, 0.0);
                backend.draw_polygon(&points, fill_opt, stroke_opt, stroke_width, opacity);
            }
            PointShape::Triangle => {
                // Triangle pointing up (rotate by -90 degrees)
                let points = self.calculate_polygon(px, py, size * 1.1, 3, -std::f64::consts::FRAC_PI_2);
                backend.draw_polygon(&points, fill_opt, stroke_opt, stroke_width, opacity);
            }
            PointShape::Pentagon => {
                let points = self.calculate_polygon(px, py, size, 5, -std::f64::consts::FRAC_PI_2);
                backend.draw_polygon(&points, fill_opt, stroke_opt, stroke_width, opacity);
            }
            PointShape::Hexagon => {
                let points = self.calculate_polygon(px, py, size, 6, 0.0);
                backend.draw_polygon(&points, fill_opt, stroke_opt, stroke_width, opacity);
            }
            PointShape::Octagon => {
                let points = self.calculate_polygon(px, py, size, 8, std::f64::consts::FRAC_PI_8);
                backend.draw_polygon(&points, fill_opt, stroke_opt, stroke_width, opacity);
            }
            PointShape::Star => {
                let points = self.calculate_star(px, py, size * 1.2, size * 0.5, 5);
                backend.draw_polygon(&points, fill_opt, stroke_opt, stroke_width, opacity);
            }
        }
    }

    /// Helper to calculate vertices for a regular N-sided polygon.
    /// * `radius`: Distance from center to vertices.
    /// * `sides`: Number of vertices.
    /// * `rotation`: Initial rotation in radians.
    fn calculate_polygon(&self, cx: f64, cy: f64, radius: f64, sides: usize, rotation: f64) -> Vec<(f64, f64)> {
        (0..sides)
            .map(|i| {
                let angle = rotation + 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
                (cx + radius * angle.cos(), cy + radius * angle.sin())
            })
            .collect()
    }

    /// Helper to calculate vertices for a star shape.
    /// * `outer_r`: Distance to outer points.
    /// * `inner_r`: Distance to inner "valleys".
    /// * `points`: Number of star points.
    fn calculate_star(&self, cx: f64, cy: f64, outer_r: f64, inner_r: f64, points: usize) -> Vec<(f64, f64)> {
        let total_points = points * 2;
        (0..total_points)
            .map(|i| {
                let angle = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * (i as f64) / (points as f64);
                let r = if i % 2 == 0 { outer_r } else { inner_r };
                (cx + r * angle.cos(), cy + r * angle.sin())
            })
            .collect()
    }
}

// ============================================================================
// LEGEND RENDERING
// ============================================================================

impl LegendRenderer for Chart<MarkPoint> {
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &crate::theme::Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Delegates to specialized sub-renderers for legend components
        crate::render::colorbar_renderer::render_colorbar(svg, self, theme, context)?;
        crate::render::color_legend_renderer::render_color_legend(svg, self, theme, context)?;
        crate::render::size_legend_renderer::render_size_legend(svg, self, theme, context)?;
        crate::render::shape_legend_renderer::render_shape_legend(svg, self, theme, context)?;

        Ok(())
    }
}