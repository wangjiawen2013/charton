use crate::core::layer::{MarkRenderer, RenderBackend};
use crate::core::context::SharedRenderingContext;
use crate::chart::Chart;
use crate::mark::point::MarkPoint;
use crate::scale::Scale;
use crate::error::ChartonError;
use crate::visual::shape::PointShape;
use crate::visual::color::SingleColor;
use itertools::izip;

// ============================================================================
// MARK RENDERING (The main data-to-geometry loop)
// ============================================================================

impl MarkRenderer for Chart<MarkPoint> {
    /// Orchestrates the transformation of raw data rows into visual point geometries.
    ///
    /// Updated to use SingleColor for aesthetics, ensuring that "none" states
    /// and complex colors are handled via structured objects rather than magic strings.
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

        // --- STEP 1: POSITION NORMALIZATION ---
        // Project raw data into the [0, 1] normalized unit space of the panel.
        let x_series = df_source.column(&x_enc.field)?;
        let y_series = df_source.column(&y_enc.field)?;

        let x_norms = x_enc.scale.as_ref().unwrap_or(&Scale::Linear)
            .normalize_series(context.coord.get_x_scale(), &x_series)?;
        
        let y_norms = y_enc.scale.as_ref().unwrap_or(&Scale::Linear)
            .normalize_series(context.coord.get_y_scale(), &y_series)?;

        // --- STEP 2: COLOR MAPPING ---
        // Resolve either a data-driven color scale or a static mark color.
        // The iterator now yields SingleColor objects directly.
        let color_iter: Box<dyn Iterator<Item = SingleColor>> = if let Some(ref mapping) = context.aesthetics.color {
            let s = df_source.column(&mapping.field)?;
            let logical_max = mapping.scale_impl.logical_max();
            let norms = mapping.scale_type.normalize_series(mapping.scale_impl.as_ref(), &s)?;
            
            let color_vec: Vec<SingleColor> = norms.into_iter()
                .map(|opt_n| mapping.mapper.map_to_color(opt_n.unwrap_or(0.0), logical_max))
                .collect();
            Box::new(color_vec.into_iter())
        } else {
            // Use the Mark configuration's color or fallback to black.
            let default_c = mark_config.color.clone();
            Box::new(std::iter::repeat(default_c))
        };

        // --- STEP 3: SHAPE MAPPING ---
        let shape_iter: Box<dyn Iterator<Item = PointShape>> = if let Some(ref mapping) = context.aesthetics.shape {
            let s = df_source.column(&mapping.field)?;
            let logical_max = mapping.scale_impl.logical_max();
            let norms = mapping.scale_type.normalize_series(mapping.scale_impl.as_ref(), &s)?;
            
            let shape_vec: Vec<PointShape> = norms.into_iter()
                .map(|opt_n| mapping.mapper.map_to_shape(opt_n.unwrap_or(0.0), logical_max))
                .collect();
            Box::new(shape_vec.into_iter())
        } else {
            Box::new(std::iter::repeat(mark_config.shape.clone()))
        };

        // --- STEP 4: SIZE MAPPING ---
        let size_iter: Box<dyn Iterator<Item = f32>> = if let Some(ref mapping) = context.aesthetics.size {
            let s = df_source.column(&mapping.field)?;
            let norms = mapping.scale_type.normalize_series(mapping.scale_impl.as_ref(), &s)?;
            
            let size_vec: Vec<f32> = norms.into_iter()
                .map(|opt_n| mapping.mapper.map_to_size(opt_n.unwrap_or(0.0)) as f32)
                .collect();
            Box::new(size_vec.into_iter())
        } else {
            Box::new(std::iter::repeat(mark_config.size as f32))
        };

        // --- STEP 5: MASTER PROJECTION & RENDERING ---
        // Prepare the fallback stroke color (default to "none" if not specified)
        let stroke_color = mark_config.stroke.clone();

        for (x_n, y_n, fill_color, current_shape, size) in izip!(
            x_norms.into_iter(), 
            y_norms.into_iter(), 
            color_iter, 
            shape_iter,
            size_iter
        ) {
            let x_norm = x_n.unwrap_or(0.0) as f32;
            let y_norm = y_n.unwrap_or(0.0) as f32;
            
            // Transform unit coordinates [0, 1] to absolute panel pixels.
            let (px, py) = context.transform(x_norm, y_norm);

            self.emit_draw_call(
                backend,
                &current_shape,
                px, py,
                size,
                &fill_color,
                &stroke_color,
                mark_config.stroke_width as f32,
                mark_config.opacity as f32
            );
        }

        Ok(())
    }
}

// ============================================================================
// GEOMETRY DISPATCH (Private helper)
// ============================================================================

impl Chart<MarkPoint> {
    /// Dispatches the appropriate backend draw call for the given PointShape.
    /// 
    /// Updated to match RenderBackend's non-optional &SingleColor signatures.
    fn emit_draw_call(
        &self,
        backend: &mut dyn RenderBackend,
        shape: &PointShape,
        px: f32,
        py: f32,
        size: f32,
        fill: &SingleColor,
        stroke: &SingleColor,
        stroke_width: f32,
        opacity: f32,
    ) {
        match shape {
            PointShape::Circle => {
                backend.draw_circle(px, py, size, fill, stroke, stroke_width, opacity);
            }
            PointShape::Square => {
                let side = size * 2.0;
                backend.draw_rect(px - size, py - size, side, side, fill, stroke, stroke_width, opacity);
            }
            PointShape::Diamond => {
                let points = self.calculate_polygon(px, py, size * 1.2, 4, 0.0);
                backend.draw_polygon(&points, fill, stroke, stroke_width, opacity);
            }
            PointShape::Triangle => {
                let points = self.calculate_polygon(px, py, size * 1.1, 3, -std::f32::consts::FRAC_PI_2);
                backend.draw_polygon(&points, fill, stroke, stroke_width, opacity);
            }
            PointShape::Pentagon => {
                let points = self.calculate_polygon(px, py, size, 5, -std::f32::consts::FRAC_PI_2);
                backend.draw_polygon(&points, fill, stroke, stroke_width, opacity);
            }
            PointShape::Hexagon => {
                let points = self.calculate_polygon(px, py, size, 6, 0.0);
                backend.draw_polygon(&points, fill, stroke, stroke_width, opacity);
            }
            PointShape::Octagon => {
                let points = self.calculate_polygon(px, py, size, 8, std::f32::consts::FRAC_PI_8);
                backend.draw_polygon(&points, fill, stroke, stroke_width, opacity);
            }
            PointShape::Star => {
                let points = self.calculate_star(px, py, size * 1.2, size * 0.5, 5);
                backend.draw_polygon(&points, fill, stroke, stroke_width, opacity);
            }
        }
    }

    /// Computes vertices for regular polygons using f32 for performance.
    fn calculate_polygon(&self, cx: f32, cy: f32, radius: f32, sides: usize, rotation: f32) -> Vec<(f32, f32)> {
        (0..sides)
            .map(|i| {
                let angle = rotation + 2.0 * std::f32::consts::PI * (i as f32) / (sides as f32);
                (cx + radius * angle.cos(), cy + radius * angle.sin())
            })
            .collect()
    }

    /// Computes vertices for a star shape.
    fn calculate_star(&self, cx: f32, cy: f32, outer_r: f32, inner_r: f32, points: usize) -> Vec<(f32, f32)> {
        let total_points = points * 2;
        (0..total_points)
            .map(|i| {
                let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * (i as f32) / (points as f32);
                let r = if i % 2 == 0 { outer_r } else { inner_r };
                (cx + r * angle.cos(), cy + r * angle.sin())
            })
            .collect()
    }
}