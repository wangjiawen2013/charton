use crate::core::layer::{MarkRenderer, RenderBackend};
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
        // Vectorized normalization: Processes the entire column into a [0, 1] range.
        let x_series = df_source.column(&x_enc.field)?;
        let y_series = df_source.column(&y_enc.field)?;

        let x_norms = x_enc.scale.as_ref().unwrap_or(&Scale::Linear)
            .normalize_series(context.coord.get_x_scale(), &x_series)?;
        
        let y_norms = y_enc.scale.as_ref().unwrap_or(&Scale::Linear)
            .normalize_series(context.coord.get_y_scale(), &y_series)?;

        // --- 2. COLOR AESTHETIC ITERATOR ---
        // Maps normalized values to specific color strings.
        let color_iter: Box<dyn Iterator<Item = String>> = if let (Some(enc), Some((scale_trait, mapper))) = (&self.encoding.color, &context.aesthetics.color) {
            let s = df_source.column(&enc.field)?;
            let scale_type = enc.scale.as_ref().unwrap_or(&Scale::Discrete);
            let logical_max = scale_trait.logical_max();

            let norms = scale_type.normalize_series(scale_trait.as_ref(), &s)?;
            
            // Optimization: Process normalization and mapping synchronously. 
            // Collecting to Vec avoids complex lifetime issues with ScaleTrait references.
            let color_vec: Vec<String> = norms.into_iter().map(|opt_n| {
                mapper.map_to_color(opt_n.unwrap_or(0.0), logical_max)
            }).collect();

            Box::new(color_vec.into_iter())
        } else {
            let default_c = mark_config.color.as_ref().map(|c| c.get_color()).unwrap_or_else(|| "black".into());
            Box::new(std::iter::repeat(default_c))
        };

        // --- 3 SHAPE AESTHETIC ITERATOR ---
        // Maps categorical data values to specific geometric PointShapes.
        let shape_iter: Box<dyn Iterator<Item = PointShape>> = if let (Some(enc), Some((scale_trait, mapper))) = (&self.encoding.shape, &context.aesthetics.shape) {
            let s = df_source.column(&enc.field)?;
            
            // If enc.scale is a direct Scale enum (not Option), use its reference.
            // If it is Option<Scale>, use .as_ref().unwrap_or(&Scale::Discrete).
            // Based on your feedback, we'll treat it as a direct Scale enum reference:
            let scale_type = &enc.scale; 
            
            // logical_max is needed by map_to_shape(norm, logical_max)
            let logical_max = scale_trait.logical_max();

            // Perform normalization: scale_trait is Box<dyn ScaleTrait>, 
            // so we use .as_ref() to get &dyn ScaleTrait for the method call.
            let norms = scale_type.normalize_series(scale_trait.as_ref(), &s)?;
            
            // Map the normalized values [0.0, 1.0] to PointShape enums.
            let shape_vec: Vec<PointShape> = norms.into_iter().map(|opt_n| {
                mapper.map_to_shape(opt_n.unwrap_or(0.0), logical_max)
            }).collect();

            Box::new(shape_vec.into_iter())
        } else {
            // Default: use the static shape from mark_config (e.g., Circle) for all points.
            Box::new(std::iter::repeat(mark_config.shape.clone()))
        };

        // --- 4. SIZE AESTHETIC ITERATOR ---
        // Maps normalized values to physical point sizes.
        let size_iter: Box<dyn Iterator<Item = f64>> = if let (Some(enc), Some((scale_trait, mapper))) = (&self.encoding.size, &context.aesthetics.size) {
            let s = df_source.column(&enc.field)?;
            let scale_type = &enc.scale; 

            let norms = scale_type.normalize_series(scale_trait.as_ref(), &s)?;
            
            // Process size mapping synchronously into an owned collection.
            let size_vec: Vec<f64> = norms.into_iter().map(|opt_n| {
                mapper.map_to_size(opt_n.unwrap_or(0.0))
            }).collect();

            Box::new(size_vec.into_iter())
        } else {
            Box::new(std::iter::repeat(mark_config.size))
        };

        // --- 5. MASTER RENDER LOOP ---
        // Zips all aesthetic streams (X, Y, Color, Size, Shape) together.
        // This ensures each data row's properties are synchronized during projection.
        for (x_n, y_n, fill_color, current_shape, size) in izip!(
            x_norms.into_iter(), 
            y_norms.into_iter(), 
            color_iter, 
            shape_iter,
            size_iter
        ) {
            let x_norm = x_n.unwrap_or(0.0);
            let y_norm = y_n.unwrap_or(0.0);
            
            // Transform normalized [0, 1] coordinates to absolute pixel positions (px, py).
            // This accounts for padding, axis directions, and panel dimensions.
            let (px, py) = context.transform(x_norm, y_norm);

            self.emit_draw_call(
                backend,
                &current_shape,
                px, py,
                size,
                &fill_color,
                // Stroke color falls back to "none" if not explicitly configured in the mark.
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
    /// Centers and renders the chosen geometric shape at (px, py).
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

    /// Calculates vertices for regular shapes.
    fn calculate_polygon(&self, cx: f64, cy: f64, radius: f64, sides: usize, rotation: f64) -> Vec<(f64, f64)> {
        (0..sides)
            .map(|i| {
                let angle = rotation + 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
                (cx + radius * angle.cos(), cy + radius * angle.sin())
            })
            .collect()
    }

    /// Calculates vertices for star-burst shapes.
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