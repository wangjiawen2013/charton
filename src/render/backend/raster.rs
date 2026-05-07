use crate::Precision;
use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PolygonConfig, RectConfig,
    RenderBackend, TextConfig,
};
use crate::visual::color::SingleColor;
use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Rect as SkiaRect, Stroke,
    Transform,
};

/// A high-performance raster rendering backend powered by tiny-skia.
///
/// This backend uses `crate::Precision` (f32) for all coordinate and
/// color opacity values to ensure maximum compatibility with GPU-native
/// types and memory efficiency.
pub struct RasterBackend<'a> {
    /// The target pixel buffer to draw upon.
    pub pixmap: &'a mut Pixmap,
    /// Global transformation matrix, typically used for DPI scaling.
    pub transform: Transform,
}

impl<'a> RasterBackend<'a> {
    /// Creates a new RasterBackend.
    ///
    /// # Arguments
    /// * `pixmap` - The pixel buffer to render into.
    /// * `scale` - The DPI scale factor (e.g., 2.0 for high-res screens).
    pub fn new(pixmap: &'a mut Pixmap, scale: Precision) -> Self {
        Self {
            pixmap,
            transform: Transform::from_scale(scale, scale),
        }
    }

    /// Converts a `SingleColor` and a `Precision` opacity into a tiny-skia `Color`.
    /// Returns `None` if the color is set to "none".
    #[inline]
    fn to_skia_color(&self, color: &SingleColor, opacity: Precision) -> Option<Color> {
        if color.is_none() {
            return None;
        }

        let rgba = color.rgba();
        Color::from_rgba(rgba[0], rgba[1], rgba[2], rgba[3] * opacity)
    }

    /// Calculates the precise width of a text string using font metrics.
    fn get_precise_width(&self, text: &str, scale: PxScale, font: &FontArc) -> Precision {
        let scaled_font = font.as_scaled(scale);
        let mut width: Precision = 0.0;
        let mut last_glyph_id = None;

        for c in text.chars() {
            let glyph_id = font.glyph_id(c);
            if let Some(last_id) = last_glyph_id {
                width += scaled_font.kern(last_id, glyph_id) as Precision;
            }
            width += scaled_font.h_advance(glyph_id) as Precision;
            last_glyph_id = Some(glyph_id);
        }
        width
    }
}

impl<'a> RenderBackend for RasterBackend<'a> {
    fn draw_circle(&mut self, config: CircleConfig) {
        // 1. Early exit if nothing to draw, matching SVG backend logic
        if config.fill.is_none() && config.stroke.is_none() {
            return;
        }

        let mut pb = PathBuilder::new();
        pb.push_circle(config.x, config.y, config.radius);

        if let Some(path) = pb.finish() {
            // 2. Render Fill
            if let Some(c) = self.to_skia_color(&config.fill, config.opacity) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;

                // Note: The last parameter is the clip mask.
                // If you implement clipping later, this 'None' should be updated.
                self.pixmap
                    .fill_path(&path, &paint, FillRule::Winding, self.transform, None);
            }

            // 3. Render Stroke
            if let Some(c) = self.to_skia_color(&config.stroke, config.opacity) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;

                let stroke = Stroke {
                    width: config.stroke_width,
                    // Note: Default SVG circle strokes don't have dash,
                    // but if CircleConfig had a dash field, it would go here.
                    ..Default::default()
                };

                self.pixmap
                    .stroke_path(&path, &paint, &stroke, self.transform, None);
            }
        }
    }

    fn draw_line(&mut self, config: LineConfig) {
        // 1. Early exit: if color is None or line is invisible, do nothing.
        if config.color.is_none() || config.width <= 0.0 {
            return;
        }

        let mut pb = PathBuilder::new();
        pb.move_to(config.x1, config.y1);
        pb.line_to(config.x2, config.y2);

        if let Some(path) = pb.finish() {
            if let Some(c) = self.to_skia_color(&config.color, config.opacity) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;

                let mut stroke = Stroke {
                    width: config.width,
                    line_cap: LineCap::Butt, // Aligns with SVG default
                    ..Default::default()
                };

                // 2. Handle dash array for dashed lines (e.g., grid lines)
                if !config.dash.is_empty() {
                    // Use the existing dash Vec directly
                    stroke.dash = tiny_skia::StrokeDash::new(config.dash, 0.0);
                }

                self.pixmap
                    .stroke_path(&path, &paint, &stroke, self.transform, None);
            }
        }
    }

    fn draw_rect(&mut self, config: RectConfig) {
        // 1. Early exit: Matches SVG backend's combined check
        if config.fill.is_none() && config.stroke.is_none() {
            return;
        }

        // Create the rectangle primitive
        if let Some(rect) = SkiaRect::from_xywh(config.x, config.y, config.width, config.height) {
            // 2. Render Fill: Uses config.opacity as per SVG fill-opacity
            if let Some(c) = self.to_skia_color(&config.fill, config.opacity) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;
                // Optimized direct rect filling
                self.pixmap.fill_rect(rect, &paint, self.transform, None);
            }

            // 3. Render Stroke: stroke-opacity is intentionally 1.0
            if let Some(c) = self.to_skia_color(&config.stroke, 1.0) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;

                let stroke = Stroke {
                    width: config.stroke_width,
                    line_join: LineJoin::Miter, // Default SVG behavior
                    ..Default::default()
                };

                // Stroke requires converting the rect to a path
                let path = PathBuilder::from_rect(rect);
                self.pixmap
                    .stroke_path(&path, &paint, &stroke, self.transform, None);
            }
        }
    }

    fn draw_path(&mut self, config: PathConfig) {
        // 1. Early exit: Matches SVG backend logic
        if config.points.is_empty() || config.stroke.is_none() {
            return;
        }

        // 2. Build Path
        let mut pb = PathBuilder::new();
        for (i, (px, py)) in config.points.iter().enumerate() {
            if i == 0 {
                pb.move_to(*px, *py);
            } else {
                pb.line_to(*px, *py);
            }
        }

        if let Some(path) = pb.finish() {
            // 3. Render Stroke (fill is "none" in SVG, so we only stroke)
            if let Some(c) = self.to_skia_color(&config.stroke, config.opacity) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;

                let mut stroke = Stroke {
                    width: config.stroke_width,
                    // IMPORTANT: Align with SVG's stroke-linejoin="round" and stroke-linecap="round"
                    line_join: LineJoin::Round,
                    line_cap: LineCap::Round,
                    ..Default::default()
                };

                // 4. Handle Dash Array
                if !config.dash.is_empty() {
                    stroke.dash = tiny_skia::StrokeDash::new(config.dash, 0.0);
                }

                self.pixmap
                    .stroke_path(&path, &paint, &stroke, self.transform, None);
            }
        }
    }

    fn draw_polygon(&mut self, config: PolygonConfig) {
        // 1. Early exit if no points, matching SVG logic
        if config.points.is_empty() {
            return;
        }

        // 2. Build Path
        let mut pb = PathBuilder::new();
        for (i, (px, py)) in config.points.iter().enumerate() {
            if i == 0 {
                pb.move_to(*px, *py);
            } else {
                pb.line_to(*px, *py);
            }
        }
        // IMPORTANT: SVG polygon automatically closes the path
        pb.close();

        if let Some(path) = pb.finish() {
            // 3. Render Fill: Use fill_opacity
            if let Some(c) = self.to_skia_color(&config.fill, config.fill_opacity) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;
                self.pixmap
                    .fill_path(&path, &paint, FillRule::Winding, self.transform, None);
            }

            // 4. Render Stroke: Use stroke_opacity
            if let Some(c) = self.to_skia_color(&config.stroke, config.stroke_opacity) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;

                let stroke = Stroke {
                    width: config.stroke_width,
                    // Standard polygon strokes usually use Miter join
                    line_join: LineJoin::Miter,
                    ..Default::default()
                };

                self.pixmap
                    .stroke_path(&path, &paint, &stroke, self.transform, None);
            }
        }
    }

    fn draw_text(&mut self, config: TextConfig) {
        // 1. Safety check: avoid unnecessary processing for empty text or missing colors
        if config.color.is_none() || config.text.is_empty() {
            return;
        }

        // --- VECTOR TUNING: STEM DARKENING ---
        // Pure vector rendering often looks "thin" on digital screens because anti-aliasing
        // softens the edges. Adding a very fine stroke (0.2px) mimics "Stem Darkening", 
        // a technique used by high-end engines like FreeType to make text look bolder and more legible.
        const STEM_DARKENING_WIDTH: f32 = 0.01; 

        // Load font and scale it to the requested pixel size
        let font = crate::core::utils::get_raster_font(&config.font_family);
        let scale = PxScale::from(config.font_size);
        let scaled_font = font.as_scaled(scale);

        // Initial cursor coordinates (using f32 for sub-pixel precision to maintain "vector feel")
        let mut x = config.x as f32;
        let mut y = config.y as f32;

        // 2. LAYOUT CALCULATION: Handle text alignment (Start, Middle, End)
        let width = self.get_precise_width(&config.text, scale, &font);
        match config.text_anchor.as_str() {
            "middle" => x -= width / 2.0,
            "end" => x -= width,
            _ => {}
        }

        // 3. BASELINE CALCULATION: Handle vertical positioning (Dominant Baseline)
        let ascent = scaled_font.ascent();
        let descent = scaled_font.descent();
        match config.dominant_baseline.as_str() {
            "hanging" => y += ascent,
            "central" | "middle" => y += (ascent - descent) / 2.0 - descent,
            _ => {} 
        }

        // Prepare Skia-style paint and color
        let base_color = self.to_skia_color(&config.color, config.opacity).unwrap();
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(base_color);
        paint.anti_alias = true; // Crucial for smooth, non-pixelated vector edges

        // Constants for mapping font units (EM) to screen pixels
        let mut last_glyph_id = None;
        let units_per_em = font.units_per_em().unwrap_or(1000.0) as f32;
        let font_to_px_scale = (config.font_size as f32) / units_per_em;

        // Iterate through each character in the string
        for c in config.text.chars() {
            let glyph_id = font.glyph_id(c);
            
            // Apply Kerning: Adjust horizontal spacing between specific pairs (e.g., 'AV')
            if let Some(last_id) = last_glyph_id {
                x += scaled_font.kern(last_id, glyph_id);
            }

            // Retrieve the vector outline for the current glyph
            if let Some(outline) = font.outline(glyph_id) {
                let mut pb = tiny_skia::PathBuilder::new();
                
                // Helper closure to convert font-space points to screen-space pixels
                // Note: We flip the Y coordinate because font space is Y-up and screen space is Y-down.
                let map_p = |p: ab_glyph::Point| (p.x * font_to_px_scale, -p.y * font_to_px_scale);

                // --- CRITICAL FIX: PATH CONTINUITY ---
                // To fill a shape, the path must be a continuous series of connected lines/curves.
                // We track 'current_pen_pos' to avoid redundant 'move_to' calls, which break the shape.
                let mut current_pen_pos: Option<ab_glyph::Point> = None;

                for curve in outline.curves {
                    // Determine the start and end points of the current segment
                    let (start_pt, end_pt) = match curve {
                        ab_glyph::OutlineCurve::Line(p1, p2) => (p1, p2),
                        ab_glyph::OutlineCurve::Quad(p1, _, p3) => (p1, p3),
                        ab_glyph::OutlineCurve::Cubic(p1, _, _, p4) => (p1, p4),
                    };

                    // Only "lift the pen" and move if the new segment doesn't start where the last one ended.
                    // This ensures we create closed 'contours' that can be filled with color.
                    if current_pen_pos != Some(start_pt) {
                        let (sx, sy) = map_p(start_pt);
                        pb.move_to(sx, sy);
                    }

                    // Add the specific geometric primitive to the path
                    match curve {
                        ab_glyph::OutlineCurve::Line(_, p2) => {
                            let (px, py) = map_p(p2);
                            pb.line_to(px, py);
                        }
                        ab_glyph::OutlineCurve::Quad(_, p2, p3) => {
                            let (p2x, p2y) = map_p(p2);
                            let (p3x, p3y) = map_p(p3);
                            pb.quad_to(p2x, p2y, p3x, p3y);
                        }
                        ab_glyph::OutlineCurve::Cubic(_, p2, p3, p4) => {
                            let (p2x, p2y) = map_p(p2);
                            let (p3x, p3y) = map_p(p3);
                            let (p4x, p4y) = map_p(p4);
                            pb.cubic_to(p2x, p2y, p3x, p3y, p4x, p4y);
                        }
                    }
                    // Update the pen position for the next iteration
                    current_pen_pos = Some(end_pt);
                }

                // Finalize the path and apply transformations
                if let Some(path) = pb.finish() {
                    let mut transform = self.transform;
                    
                    // 4. ROTATION LOGIC: Rotate around the anchor point (config.x, config.y)
                    if config.angle != 0.0 {
                        transform = transform
                            .pre_translate(config.x as f32, config.y as f32)
                            .pre_rotate(config.angle as f32)
                            .pre_translate(-(config.x as f32), -(config.y as f32));
                    }
                    
                    // Position the high-res vector path at the current cursor 'x' and 'y'
                    transform = transform.pre_translate(x, y);

                    // --- RENDERING PHASE ---
                    
                    // Pass 1: FILL (The core color)
                    // This fills the interior of the closed path. Since the path is now continuous,
                    // the interior will be solid and dark.
                    self.pixmap.fill_path(
                        &path, 
                        &paint, 
                        tiny_skia::FillRule::Winding, 
                        transform, 
                        None
                    );
                    
                    // Pass 2: STROKE (The darkening agent)
                    // We add a tiny stroke to reinforce the edges. This compensates for 
                    // LCD/OLED sub-pixel rendering effects that can make thin fonts look grey.
                    let mut stroke = tiny_skia::Stroke::default();
                    stroke.width = STEM_DARKENING_WIDTH;
                    self.pixmap.stroke_path(&path, &paint, &stroke, transform, None);
                }
            }

            // Move the cursor forward by the glyph's advance width for the next character
            x += scaled_font.h_advance(glyph_id);
            last_glyph_id = Some(glyph_id);
        }
    }

    fn draw_gradient_rect(&mut self, config: GradientRectConfig) {
        let GradientRectConfig {
            x,
            y,
            width,
            height,
            stops,
            is_vertical,
            .. // id_suffix is not needed for direct raster rendering
        } = config;

        if let Some(rect) = SkiaRect::from_xywh(x, y, width, height) {
            let mut paint = Paint::default();
            paint.anti_alias = true;

            // 1. Prepare Gradient Stops
            let skia_stops: Vec<tiny_skia::GradientStop> = stops
                .into_iter()
                .filter_map(|(offset, color)| {
                    // reuse our to_skia_color helper (opacity 1.0 for gradients usually)
                    self.to_skia_color(&color, 1.0)
                        .map(|c| tiny_skia::GradientStop::new(offset as f32, c))
                })
                .collect();

            if skia_stops.is_empty() {
                return;
            }

            // 2. Define Gradient Points (Absolute coordinates)
            // SVG (0%,0%) to (x2,y2%) mapping:
            let start = tiny_skia::Point::from_xy(x, y);
            let end = if is_vertical {
                tiny_skia::Point::from_xy(x, y + height) // Vertical: Top to Bottom
            } else {
                tiny_skia::Point::from_xy(x + width, y) // Horizontal: Left to Right
            };

            // 3. Set Shader
            if let Some(shader) = tiny_skia::LinearGradient::new(
                start,
                end,
                skia_stops,
                tiny_skia::SpreadMode::Pad, // Matches SVG default
                self.transform,             // Apply current global transform
            ) {
                paint.shader = shader;
            }

            // 4. Draw
            self.pixmap.fill_rect(rect, &paint, self.transform, None);
        }
    }
}

// 这个矢量的可做参考：
// fn draw_text(&mut self, config: TextConfig) {
//     if config.color.is_none() || config.text.is_empty() {
//         return;
//     }

//     // --- VECTOR TUNING PARAMETERS ---
//     // The width of the "ink" stroke to prevent the text from looking too thin.
//     // 0.1 to 0.25 keeps it crisp but bold.
//     const STEM_DARKENING_WIDTH: f32 = 0.15; 

//     let font = crate::core::utils::get_raster_font(&config.font_family);
//     let scale = PxScale::from(config.font_size);
//     let scaled_font = font.as_scaled(scale);

//     // 1. USE FLOATS: Remove .round() to enable sub-pixel positioning.
//     // This makes the movement and spacing feel "more vector" and fluid.
//     let mut x = config.x as f32;
//     let mut y = config.y as f32;

//     let width = self.get_precise_width(&config.text, scale, &font);
//     match config.text_anchor.as_str() {
//         "middle" => x -= width / 2.0,
//         "end" => x -= width,
//         _ => {}
//     }

//     let ascent = scaled_font.ascent();
//     let descent = scaled_font.descent();
//     match config.dominant_baseline.as_str() {
//         "hanging" => y += ascent,
//         "central" | "middle" => y += (ascent - descent) / 2.0 - descent,
//         _ => {} 
//     }

//     let base_color = self.to_skia_color(&config.color, config.opacity).unwrap();
//     let mut paint = tiny_skia::Paint::default();
//     paint.set_color(base_color);
//     paint.anti_alias = true; // Essential for the vector look.

//     let mut last_glyph_id = None;
//     let units_per_em = font.units_per_em().unwrap_or(1000.0) as f32;
//     let font_to_px_scale = (config.font_size as f32) / units_per_em;

//     for c in config.text.chars() {
//         let glyph_id = font.glyph_id(c);
//         if let Some(last_id) = last_glyph_id {
//             x += scaled_font.kern(last_id, glyph_id);
//         }

//         // 2. CONVERT TO PATH: Transform ab_glyph curves directly into tiny-skia paths.
//         if let Some(outline) = font.outline(glyph_id) {
//             let mut pb = tiny_skia::PathBuilder::new();
            
//             // Helper to map font units to screen pixels
//             let map_p = |p: ab_glyph::Point| (p.x * font_to_px_scale, -p.y * font_to_px_scale);

//             for curve in outline.curves {
//                 match curve {
//                     ab_glyph::OutlineCurve::Line(p1, p2) => {
//                         let (p1x, p1y) = map_p(p1);
//                         let (p2x, p2y) = map_p(p2);
//                         pb.move_to(p1x, p1y);
//                         pb.line_to(p2x, p2y);
//                     }
//                     ab_glyph::OutlineCurve::Quad(p1, p2, p3) => {
//                         let (p1x, p1y) = map_p(p1);
//                         let (p2x, p2y) = map_p(p2);
//                         let (p3x, p3y) = map_p(p3);
//                         pb.move_to(p1x, p1y);
//                         pb.quad_to(p2x, p2y, p3x, p3y);
//                     }
//                     ab_glyph::OutlineCurve::Cubic(p1, p2, p3, p4) => {
//                         let (p1x, p1y) = map_p(p1);
//                         let (p2x, p2y) = map_p(p2);
//                         let (p3x, p3y) = map_p(p3);
//                         let (p4x, p4y) = map_p(p4);
//                         pb.move_to(p1x, p1y);
//                         pb.cubic_to(p2x, p2y, p3x, p3y, p4x, p4y);
//                     }
//                 }
//             }

//             if let Some(path) = pb.finish() {
//                 let mut transform = self.transform;
//                 if config.angle != 0.0 {
//                     transform = transform
//                         .pre_translate(config.x as f32, config.y as f32)
//                         .pre_rotate(config.angle as f32)
//                         .pre_translate(-(config.x as f32), -(config.y as f32));
//                 }
//                 // Use the floating-point x, y for the final transform
//                 transform = transform.pre_translate(x, y);

//                 // 3. DUAL RENDERING: Fill the shape AND add a subtle stroke.
//                 // This gives the "vector" look - perfect edges with strong presence.
//                 self.pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
                
//                 let mut stroke = tiny_skia::Stroke::default();
//                 stroke.width = STEM_DARKENING_WIDTH;
//                 self.pixmap.stroke_path(&path, &paint, &stroke, transform, None);
//             }
//         }

//         x += scaled_font.h_advance(glyph_id);
//         last_glyph_id = Some(glyph_id);
//     }
// }
