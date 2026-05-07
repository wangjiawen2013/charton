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
        // 1. Pre-checks
        if config.color.is_none() || config.text.is_empty() {
            return;
        }

        // 2. Font Resolution
        let font = crate::core::utils::get_raster_font(&config.font_family);
        let scale = PxScale::from(config.font_size);
        let scaled_font = font.as_scaled(scale);

        // 3. Layout Calculations
        let mut x = config.x;
        let mut y = config.y; 

        // Horizontal Alignment
        let width = self.get_precise_width(&config.text, scale, &font);
        match config.text_anchor.as_str() {
            "middle" => x -= width / 2.0,
            "end" => x -= width,
            _ => {} 
        }

        // Vertical Alignment
        let ascent = scaled_font.ascent();
        let descent = scaled_font.descent();
        let text_height = ascent - descent;

        match config.dominant_baseline.as_str() {
            "hanging" => y += ascent,
            "central" | "middle" => y += (text_height / 2.0) - descent,
            _ => {} // Alphabetic: y stays at baseline
        }

        // 4. Paint Setup
        let mut paint = Paint::default();
        if let Some(c) = self.to_skia_color(&config.color, config.opacity) {
            paint.set_color(c);
            paint.anti_alias = true;
        } else {
            return;
        }

        let rot_x = config.x;
        let rot_y = config.y;

        // 5. Glyph Rendering Loop
        let mut last_glyph_id = None;
        for c in config.text.chars() {
            let glyph_id = font.glyph_id(c);

            // Apply Kerning
            if let Some(last_id) = last_glyph_id {
                x += scaled_font.kern(last_id, glyph_id) as Precision;
            }

            // Get the Raw Vector Outline (Font Units)
            if let Some(outline) = font.outline(glyph_id) {
                let mut pb = PathBuilder::new();

                for curve in outline.curves {
                    match curve {
                        ab_glyph::OutlineCurve::Line(p1, p2) => {
                            pb.move_to(p1.x, p1.y);
                            pb.line_to(p2.x, p2.y);
                        }
                        ab_glyph::OutlineCurve::Quad(p1, p2, p3) => {
                            pb.move_to(p1.x, p1.y);
                            pb.quad_to(p2.x, p2.y, p3.x, p3.y);
                        }
                        ab_glyph::OutlineCurve::Cubic(p1, p2, p3, p4) => {
                            pb.move_to(p1.x, p1.y);
                            pb.cubic_to(p2.x, p2.y, p3.x, p3.y, p4.x, p4.y);
                        }
                    }
                }

                if let Some(path) = pb.finish() {
                    let mut transform = self.transform;

                    // A. Apply Rotation around the original (config.x, config.y)
                    if config.angle != 0.0 {
                        transform = transform
                            .pre_translate(rot_x as f32, rot_y as f32)
                            .pre_rotate(config.angle as f32)
                            .pre_translate(-(rot_x as f32), -(rot_y as f32));
                    }

                    // B. Position the glyph at current cursor (x, y)
                    transform = transform.pre_translate(x.round() as f32, y.round() as f32);

                    // C. Scale from "Font Units" to "Pixels"
                    // Most fonts use 1000 or 2048 units per EM. 
                    // We divide by this to get a normalized 0..1 scale, then multiply by font_size.
                    let units_per_em = font.units_per_em().unwrap_or(1000.0) as f32;
                    let s = (config.font_size as f32) / units_per_em;
                    
                    // Flip Y (-s) because font coordinates are Y-up, Skia is Y-down.
                    transform = transform.pre_scale(s, -s);

                    self.pixmap.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        transform,
                        None,
                    );
                }
            }

            // Advance cursor to the next character
            x += scaled_font.h_advance(glyph_id) as Precision;
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
