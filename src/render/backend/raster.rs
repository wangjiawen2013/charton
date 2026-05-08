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
    if config.color.is_none() || config.text.is_empty() {
        return;
    }

    let font = crate::core::utils::get_raster_font(&config.font_family);
    let scale = PxScale::from(config.font_size);
    let scaled_font = font.as_scaled(scale);

    let anchor_x = config.x as f32;
    let anchor_y = config.y as f32;

    // 1. 严格计算水平宽度 (用于 text-anchor)
    let width = self.get_precise_width(&config.text, scale, &font);
    const TRACKING: f32 = 0.3;
    let total_tracking = if config.text.len() > 1 { (config.text.len() - 1) as f32 * TRACKING } else { 0.0 };
    let full_width = width + total_tracking;

    let mut dx = 0.0;
    match config.text_anchor.as_str() {
        "middle" => dx -= full_width / 2.0,
        "end" => dx -= full_width,
        _ => {}
    }

    // 2. 严格对齐 SVG dominant-baseline
    // 在 SVG 中，默认 y 就是 baseline，所以 dy = 0
    let mut dy = 0.0;
    let ascent = scaled_font.ascent();
    let descent = scaled_font.descent();

    match config.dominant_baseline.as_str() {
        "hanging" => dy += ascent, 
        "central" | "middle" => {
            // 这是关键：SVG 的 middle 是相对于 Baseline 的偏移
            // 通常是 (ascent + descent) / 2，但注意 descent 是负值
            dy += (ascent + descent) / 2.0; 
        }
        _ => {} // alphabetic: dy = 0 (baseline 对齐)
    }

    // 3. 构建与 SVG 相同的变换矩阵
    // SVG: transform="rotate(angle, x, y)"
    let mut global_transform = self.transform;
    if config.angle != 0.0 {
        global_transform = global_transform
            .pre_translate(anchor_x, anchor_y)
            .pre_rotate(config.angle as f32)
            .pre_translate(-anchor_x, -anchor_y);
    }

    let mut current_x = anchor_x + dx;
    let draw_y = anchor_y + dy;

    let base_color = self.to_skia_color(&config.color, config.opacity).unwrap();
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(base_color);
    paint.anti_alias = true;

    let units_per_em = font.units_per_em().unwrap_or(1000.0) as f32;
    let font_to_px = (config.font_size as f32) / units_per_em;

    let mut last_glyph_id = None;

    for c in config.text.chars() {
        let glyph_id = font.glyph_id(c);
        if let Some(last_id) = last_glyph_id {
            current_x += scaled_font.kern(last_id, glyph_id);
        }

        if let Some(outline) = font.outline(glyph_id) {
            let mut pb = tiny_skia::PathBuilder::new();
            // 注意：这里 p.y 取负是因为 ab_glyph 坐标系是向上增长的
            let map_p = |p: ab_glyph::Point| (p.x * font_to_px, -p.y * font_to_px);

            let mut current_pen: Option<ab_glyph::Point> = None;
            for curve in outline.curves {
                let (start, end) = match curve {
                    ab_glyph::OutlineCurve::Line(p1, p2) => (p1, p2),
                    ab_glyph::OutlineCurve::Quad(p1, _, p3) => (p1, p3),
                    ab_glyph::OutlineCurve::Cubic(p1, _, _, p4) => (p1, p4),
                };

                if current_pen != Some(start) {
                    let (sx, sy) = map_p(start);
                    pb.move_to(sx, sy);
                }

                match curve {
                    ab_glyph::OutlineCurve::Line(_, p2) => { let (px, py) = map_p(p2); pb.line_to(px, py); }
                    ab_glyph::OutlineCurve::Quad(_, p2, p3) => {
                        let (p2x, p2y) = map_p(p2); let (p3x, p3y) = map_p(p3);
                        pb.quad_to(p2x, p2y, p3x, p3y);
                    }
                    ab_glyph::OutlineCurve::Cubic(_, p2, p3, p4) => {
                        let (p2x, p2y) = map_p(p2); let (p3x, p3y) = map_p(p3);
                        let (p4x, p4y) = map_p(p4);
                        pb.cubic_to(p2x, p2y, p3x, p3y, p4x, p4y);
                    }
                }
                current_pen = Some(end);
            }

            if let Some(path) = pb.finish() {
                // 先应用旋转，再平移到当前字母的 baseline 位置
                let glyph_transform = global_transform.pre_translate(current_x, draw_y);
                self.pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, glyph_transform, None);
                
                // 极细微描边增强
                let mut stroke = tiny_skia::Stroke::default();
                stroke.width = 0.05;
                self.pixmap.stroke_path(&path, &paint, &stroke, glyph_transform, None);
            }
        }

        current_x += scaled_font.h_advance(glyph_id) + TRACKING;
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

// 这个draw_text可做参考,是另一种策略：
// fn draw_text(&mut self, config: TextConfig) {
//     // 1. Safety Guard: Skip rendering if no color or empty text
//     if config.color.is_none() || config.text.is_empty() {
//         return;
//     }

//     // --- TUNING PARAMETER: Contrast/Darkness ---
//     // Increase this to make text "blacker", decrease to make it "smoother".
//     // Suggested range: 1.2 (soft) to 1.6 (bold/sharp).
//     const CONTRAST_BOOST: f32 = 1.4;

//     let font = crate::core::utils::get_raster_font(&config.font_family);
//     let scale = PxScale::from(config.font_size);
//     let scaled_font = font.as_scaled(scale);

//     // 2. Pixel Grid Snapping
//     // Rounding the starting coordinates is crucial to prevent "blurry" vertical stems.
//     let mut x = config.x.round();
//     let mut y = config.y.round();

//     // Horizontal Alignment Calculation
//     let width = self.get_precise_width(&config.text, scale, &font);
//     match config.text_anchor.as_str() {
//         "middle" => x -= (width / 2.0).round(),
//         "end" => x -= width.round(),
//         _ => {} // Default: Start
//     }

//     // Vertical Alignment Calculation (Baseline snapping)
//     let ascent = scaled_font.ascent();
//     let descent = scaled_font.descent();
//     match config.dominant_baseline.as_str() {
//         "hanging" => y += ascent.round(),
//         "central" | "middle" => y += ((ascent - descent) / 2.0 - descent).round(),
//         _ => {} // Default: Alphabetic
//     }

//     let base_color = self.to_skia_color(&config.color, config.opacity).unwrap();

//     // 3. Setup Global Transformation (Handling SVG Rotation)
//     let mut global_transform = self.transform;
//     if config.angle != 0.0 {
//         global_transform = global_transform
//             .pre_translate(config.x as f32, config.y as f32)
//             .pre_rotate(config.angle as f32)
//             .pre_translate(-(config.x as f32), -(config.y as f32));
//     }

//     let mut last_glyph_id = None;
//     for c in config.text.chars() {
//         let glyph_id = font.glyph_id(c);

//         // Apply Kerning (adjust space between specific pairs like 'AV')
//         if let Some(last_id) = last_glyph_id {
//             x += scaled_font.kern(last_id, glyph_id).round();
//         }

//         let glyph =
//             glyph_id.with_scale_and_position(scale, ab_glyph::point(x as f32, y as f32));

//         if let Some(outlined) = font.outline_glyph(glyph) {
//             let bounds = outlined.px_bounds();
//             let w = bounds.width() as u32;
//             let h = bounds.height() as u32;

//             if w > 0 && h > 0 {
//                 // 4. Per-Glyph Rasterization
//                 // We create a tiny temporary canvas for the single character.
//                 let mut glyph_pixmap = tiny_skia::Pixmap::new(w, h).unwrap();
//                 let pixels = glyph_pixmap.pixels_mut();

//                 outlined.draw(|px, py, coverage| {
//                     if coverage <= 0.001 {
//                         return;
//                     }

//                     // --- SMOOTHNESS vs DARKNESS LOGIC ---
//                     // By multiplying coverage, we shift the anti-aliasing ramp.
//                     // This creates a "Stem Darkening" effect used by pro renderers.
//                     let alpha_factor = (coverage * CONTRAST_BOOST).min(1.0);

//                     let idx = (py as usize * w as usize) + px as usize;

//                     // Manually compute Premultiplied Alpha for maximum clarity
//                     let alpha = base_color.alpha() * alpha_factor;
//                     if let Some(c) = tiny_skia::Color::from_rgba(
//                         base_color.red(),
//                         base_color.green(),
//                         base_color.blue(),
//                         alpha,
//                     ) {
//                         pixels[idx] = c.premultiply().to_color_u8();
//                     }
//                 });

//                 // 5. Blit the Glyph to Main Canvas
//                 let mut paint = tiny_skia::PixmapPaint::default();

//                 // --- ROTATION SMOOTHNESS ---
//                 // Bilinear quality prevents "staircase" artifacts during rotation.
//                 // For 0-degree text, this performs a perfect 1:1 pixel copy.
//                 paint.quality = tiny_skia::FilterQuality::Bilinear;

//                 // Move the small glyph pixmap to its intended world position
//                 let glyph_transform =
//                     global_transform.pre_translate(bounds.min.x, bounds.min.y);

//                 self.pixmap.draw_pixmap(
//                     0,
//                     0,
//                     glyph_pixmap.as_ref(),
//                     &paint,
//                     glyph_transform,
//                     None,
//                 );
//             }
//         }

//         // Advance cursor for the next character
//         x += scaled_font.h_advance(glyph_id).round();
//         last_glyph_id = Some(glyph_id);
//     }
// }
