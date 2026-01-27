use crate::core::layer::{MarkRenderer, RenderBackend};
use crate::core::context::SharedRenderingContext;
use crate::chart::Chart;
use crate::mark::point::MarkPoint;
use crate::error::ChartonError;
use crate::visual::shape::PointShape;
use crate::visual::color::SingleColor;

// ============================================================================
// MARK RENDERING (The main data-to-geometry loop)
// ============================================================================

impl MarkRenderer for Chart<MarkPoint> {
    /// Orchestrates the transformation of raw data rows into visual point geometries.
    /// 
    /// This implementation follows a robust data-to-visual pipeline:
    /// 1. **Validation**: Uses .ok_or_else to ensure encodings exist, providing clear error context.
    /// 2. **Positioning**: Retrieves Scales from the Coordinate system and performs vectorized 
    ///    normalization using Polars for high performance.
    /// 3. **Aesthetic Resolution**: Maps data to Color, Shape, and Size using Sampler-Mapper pairs 
    ///    encapsulated within the ScaleTrait.
    /// 4. **Projection**: Transforms normalized [0, 1] units to physical panel pixels and 
    ///    dispatches draw calls to the backend.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        
        // Return early if there is no data to render to avoid unnecessary overhead.
        if df_source.df.height() == 0 { 
            return Ok(()); 
        }

        // --- STEP 1: ENCODING VALIDATION ---
        // We avoid .unwrap() to ensure the library fails gracefully with descriptive error messages.
        let x_enc = self.encoding.x.as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding is missing from specification".to_string()))?;
        let y_enc = self.encoding.y.as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y-axis encoding is missing from specification".to_string()))?;
        
        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkPoint configuration is missing".to_string()))?;

        // --- STEP 2: POSITION NORMALIZATION (Vectorized) ---
        // Extract raw data columns as Polars Series.
        let x_series = df_source.column(&x_enc.field)?;
        let y_series = df_source.column(&y_enc.field)?;

        // Access trait objects from the coordinate system (the single source of truth for layout).
        let x_scale_trait = context.coord.get_x_scale();
        let y_scale_trait = context.coord.get_y_scale();

        // Perform vectorized normalization by dispatching based on the Scale enum type 
        // returned by the trait implementation.
        let x_norms = x_scale_trait.scale_type().normalize_series(x_scale_trait, &x_series)?;
        let y_norms = y_scale_trait.scale_type().normalize_series(y_scale_trait, &y_series)?;

        // --- STEP 3: COLOR MAPPING ---
        // Resolve data-driven color scale or fallback to a static mark color.
        let color_iter: Box<dyn Iterator<Item = SingleColor>> = if let Some(ref mapping) = context.aesthetics.color {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            
            // Normalize the aesthetic series using the mapper's specific scale logic.
            let norms = s_trait.scale_type().normalize_series(s_trait, &s)?;
            let l_max = s_trait.logical_max();
            
            let color_vec: Vec<SingleColor> = norms.into_iter()
                .map(|opt_n| {
                    // Extract the visual mapper from the scale implementation.
                    s_trait.mapper()
                        .map(|m| m.map_to_color(opt_n.unwrap_or(0.0), l_max))
                        .unwrap_or_else(|| SingleColor::from("#333333"))
                })
                .collect();
            Box::new(color_vec.into_iter())
        } else {
            Box::new(std::iter::repeat(mark_config.color.clone()))
        };

        // --- STEP 4: SHAPE MAPPING ---
        let shape_iter: Box<dyn Iterator<Item = PointShape>> = if let Some(ref mapping) = context.aesthetics.shape {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            
            let norms = s_trait.scale_type().normalize_series(s_trait, &s)?;
            let l_max = s_trait.logical_max();
            
            let shape_vec: Vec<PointShape> = norms.into_iter()
                .map(|opt_n| {
                    s_trait.mapper()
                        .map(|m| m.map_to_shape(opt_n.unwrap_or(0.0), l_max))
                        .unwrap_or(PointShape::Circle)
                })
                .collect();
            Box::new(shape_vec.into_iter())
        } else {
            Box::new(std::iter::repeat(mark_config.shape.clone()))
        };

        // --- STEP 5: SIZE MAPPING ---
        let size_iter: Box<dyn Iterator<Item = f32>> = if let Some(ref mapping) = context.aesthetics.size {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            
            let norms = s_trait.scale_type().normalize_series(s_trait, &s)?;
            
            let size_vec: Vec<f32> = norms.into_iter()
                .map(|opt_n| {
                    s_trait.mapper()
                        .map(|m| m.map_to_size(opt_n.unwrap_or(0.0)) as f32)
                        .unwrap_or(mark_config.size as f32)
                })
                .collect();
            Box::new(size_vec.into_iter())
        } else {
            Box::new(std::iter::repeat(mark_config.size as f32))
        };

        // --- STEP 6: GEOMETRY PROJECTION & RENDERING ---
        let stroke_color = mark_config.stroke.clone();

        // Zip all aesthetic streams into a single loop to emit draw calls for each row.
        for ((((x_n, y_n), fill_color), current_shape), size) in x_norms.into_iter()
            .zip(y_norms.into_iter()) 
            .zip(color_iter) 
            .zip(shape_iter)
            .zip(size_iter)
        {
            // Default to 0.0 for missing data points.
            let x_norm = x_n.unwrap_or(0.0) as f32;
            let y_norm = y_n.unwrap_or(0.0) as f32;
            
            // Convert normalized [0, 1] units to physical panel pixels (e.g. SVG coordinates).
            let (px, py) = context.transform(x_norm, y_norm);

            // Emit the final command to the rendering backend (SVG, Canvas, etc.).
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