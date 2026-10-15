use crate::Precision;
use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PathTopology, PolygonConfig,
    RectConfig, RenderBackend, TextConfig,
};
use crate::error::ChartonError;
use crate::visual::color::SingleColor;
use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, Rect as SkiaRect, Stroke,
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
    /// Active clipping mask to constrain drawing within the panel viewport.
    pub clip_mask: Option<Mask>,
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
            clip_mask: None, // Initialize with no active clipping mask
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
}

impl<'a> RenderBackend for RasterBackend<'a> {
    // =========================================================================
    // STATE MACHINE SCOPE IMPLEMENTATION
    // =========================================================================

    fn begin_clip_scope(&mut self, rect: &crate::coordinate::Rect) {
        // Create an uninitialized mask matching the size of the canvas pixmap
        let width = self.pixmap.width();
        let height = self.pixmap.height();

        if let Some(mut mask) = Mask::new(width, height) {
            let mut pb = PathBuilder::new();

            // Apply the DPI scale transform to the clipping rectangle bounds
            let scale_x = self.transform.sx;
            let scale_y = self.transform.sy;

            if let Some(skia_rect) = SkiaRect::from_xywh(
                (rect.x as f32) * scale_x,
                (rect.y as f32) * scale_y,
                (rect.width as f32) * scale_x,
                (rect.height as f32) * scale_y,
            ) {
                pb.push_rect(skia_rect);
                if let Some(path) = pb.finish() {
                    // Fill the mask path with a winding rule to define the clip area
                    mask.fill_path(
                        &path,
                        FillRule::Winding,
                        true,                  // Anti-aliasing
                        Transform::identity(), // Bounds are already pre-scaled
                    );
                    self.clip_mask = Some(mask);
                }
            }
        }
    }

    fn end_clip_scope(&mut self) {
        // Drop the clipping mask to restore full canvas drawing
        self.clip_mask = None;
    }

    // =========================================================================
    // SHAPE DRAWING METHODS
    // =========================================================================

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

                self.pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    self.transform,
                    self.clip_mask.as_ref(),
                );
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

                self.pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    self.transform,
                    self.clip_mask.as_ref(),
                );
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

        if let Some(path) = pb.finish()
            && let Some(c) = self.to_skia_color(&config.color, config.opacity)
        {
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

            self.pixmap.stroke_path(
                &path,
                &paint,
                &stroke,
                self.transform,
                self.clip_mask.as_ref(),
            );
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
                self.pixmap
                    .fill_rect(rect, &paint, self.transform, self.clip_mask.as_ref());
            }

            // 3. Render Stroke: stroke-opacity is intentionally 1.0.
            //    A zero width means "no stroke" (SVG semantics); skip it so the
            //    rasterizer does not leave a hairline between adjacent cells.
            if config.stroke_width > 0.0
                && let Some(c) = self.to_skia_color(&config.stroke, 1.0)
            {
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
                self.pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    self.transform,
                    self.clip_mask.as_ref(),
                );
            }
        }
    }

    fn draw_path(&mut self, config: PathConfig) {
        // 1. Universal Early Exit: Abort only if there are no points or BOTH fill and stroke are missing.
        if config.points.is_empty() || (config.fill.is_none() && config.stroke.is_none()) {
            return;
        }

        // 2. Build Path Geometry
        let mut pb = PathBuilder::new();
        for (i, (px, py)) in config.points.iter().enumerate() {
            if i == 0 {
                pb.move_to(*px, *py);
            } else {
                pb.line_to(*px, *py);
            }
        }

        // ====================================================================
        // OPTIMIZATION 1: Auto Path Closure for Areas
        // ====================================================================
        // If the topology is Complex (e.g., an area chart) or it has a fill color,
        // explicitly close the path in tiny-skia to ensure perfect fill rendering.
        if matches!(config.topology, PathTopology::Complex) || !config.fill.is_none() {
            pb.close();
        }

        if let Some(path) = pb.finish() {
            // ====================================================================
            // OPTIMIZATION 2: Render Fill (Layer 1 equivalent)
            // ====================================================================
            if !config.fill.is_none()
                && let Some(c) = self.to_skia_color(&config.fill, config.opacity)
            {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true; // Crucial for eliminating hairline seams

                // FillRule::Winding is the standard non-zero algorithm used by SVG
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    self.transform,
                    self.clip_mask.as_ref(),
                );
            }

            // ====================================================================
            // OPTIMIZATION 3: Render Stroke (Layer 2 equivalent)
            // ====================================================================
            if !config.stroke.is_none()
                && config.stroke_width > 0.0
                && let Some(c) = self.to_skia_color(&config.stroke, config.opacity)
            {
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

                // Handle Dash Array
                if !config.dash.is_empty() {
                    stroke.dash = tiny_skia::StrokeDash::new(config.dash, 0.0);
                }

                self.pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    self.transform,
                    self.clip_mask.as_ref(),
                );
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
            if let Some(c) = self.to_skia_color(&config.fill, config.opacity) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    self.transform,
                    self.clip_mask.as_ref(),
                );
            }

            // 4. Render Stroke: stroke-opacity is intentionally 1.0 to match SVG behavior
            if let Some(c) = self.to_skia_color(&config.stroke, 1.0) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;

                let stroke = Stroke {
                    width: config.stroke_width,
                    // Standard polygon strokes usually use Miter join
                    line_join: LineJoin::Miter,
                    ..Default::default()
                };

                self.pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    self.transform,
                    self.clip_mask.as_ref(),
                );
            }
        }
    }

    fn draw_text(&mut self, config: TextConfig) -> Result<(), ChartonError> {
        if config.color.is_none() || config.text.is_empty() {
            return Ok(());
        }

        let request =
            crate::core::font::FontRequest::from_css(&config.font_weight, &config.font_style);
        let stack = crate::core::font::get_raster_fonts(&config.font_family, request)?;
        let scale = PxScale::from(config.font_size);

        // The primary face defines the line box (ascent/descent); individual
        // glyphs may come from a fallback face when the primary lacks them.
        let primary = stack.primary().as_scaled(scale);

        let anchor_x = config.x;
        let anchor_y = config.y;

        // 1. Lay out every character: face, glyph and advance, with same-face
        //    kerning. The same layout drives the anchor width and the drawing.
        let placements = stack.layout(&config.text, scale);
        let width: Precision = placements.iter().map(|g| g.kern + g.advance).sum();
        const TRACKING: f32 = 0.3;
        // Tracking is applied once per character, not once per byte, so the
        // anchor width must count characters (`placements.len()`), otherwise
        // multi-byte text such as CJK is mis-anchored.
        let total_tracking = if placements.len() > 1 {
            (placements.len() - 1) as f32 * TRACKING
        } else {
            0.0
        };
        let full_width = width + total_tracking;

        let mut dx = 0.0;
        match config.text_anchor.as_str() {
            "middle" => dx -= full_width / 2.0,
            "end" => dx -= full_width,
            _ => {}
        }

        // 2. Strictly align with SVG dominant-baseline
        // In SVG, the default y is the baseline, so dy = 0
        let mut dy = 0.0;
        let ascent = primary.ascent();
        let descent = primary.descent();

        match config.dominant_baseline.as_str() {
            "hanging" => dy += ascent,
            "central" | "middle" => {
                // Key logic: SVG's middle is an offset relative to the Baseline
                // Usually (ascent + descent) / 2, noting that descent is a negative value
                dy += (ascent + descent) / 2.0;
            }
            _ => {} // alphabetic: dy = 0 (baseline alignment)
        }

        // 3. Build the same transformation matrix as SVG
        // SVG: transform="rotate(angle, x, y)"
        let mut global_transform = self.transform;
        if config.angle != 0.0 {
            global_transform = global_transform
                .pre_translate(anchor_x, anchor_y)
                .pre_rotate(config.angle)
                .pre_translate(-anchor_x, -anchor_y);
        }

        let mut current_x = anchor_x + dx;
        let draw_y = anchor_y + dy;

        let base_color = self.to_skia_color(&config.color, config.opacity).unwrap();
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(base_color);
        paint.anti_alias = true;

        for placement in &placements {
            current_x += placement.kern;

            // Outlines are scaled with the same factor ab_glyph uses for the
            // advances that position them. See `build_glyph_path`.
            if let Some(path) = build_glyph_path(stack.face(placement.face), placement.glyph, scale)
            {
                // Apply rotation first, then translate to the current character's baseline position
                let glyph_transform = global_transform.pre_translate(current_x, draw_y);
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    glyph_transform,
                    self.clip_mask.as_ref(),
                );
            }

            current_x += placement.advance + TRACKING;
        }

        Ok(())
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
            let mut paint = Paint {
                anti_alias: true,
                ..Default::default()
            };

            // 1. Prepare Gradient Stops
            let skia_stops: Vec<tiny_skia::GradientStop> = stops
                .into_iter()
                .filter_map(|(offset, color)| {
                    // Reuse the to_skia_color helper (opacity 1.0 for gradients usually)
                    self.to_skia_color(&color, 1.0)
                        .map(|c| tiny_skia::GradientStop::new(offset, c))
                })
                .collect();

            // 2. Early return if no valid stops are available
            if skia_stops.is_empty() {
                return;
            }

            // 3. Define Gradient Points (Absolute canvas coordinates)
            let start = tiny_skia::Point::from_xy(x, y);
            let end = if skia_stops.len() == 1 {
                // If there is only 1 stop, set the end point identical to start.
                start
            } else if is_vertical {
                tiny_skia::Point::from_xy(x, y + height) // Vertical: Top to Bottom
            } else {
                tiny_skia::Point::from_xy(x + width, y) // Horizontal: Left to Right
            };

            // 4. Set Shader with Identity Transform
            // Pass tiny_skia::Transform::default() here instead of self.transform.
            // The canvas global transform will already be applied in fill_rect below.
            if let Some(shader) = tiny_skia::LinearGradient::new(
                start,
                end,
                skia_stops,
                tiny_skia::SpreadMode::Pad, // Matches SVG default behavior
                tiny_skia::Transform::default(),
            ) {
                paint.shader = shader;
            }

            // 5. Render to Pixmap
            // The self.transform matrix correctly scales both the rect geometry and the shader once.
            self.pixmap
                .fill_rect(rect, &paint, self.transform, self.clip_mask.as_ref());
        }
    }
}

/// Builds a tiny-skia path for a single glyph outline, relative to the glyph's
/// pen position (baseline at `(0, 0)`, y growing downwards).
///
/// # Why the scale factor matters
/// `ab_glyph`'s [`PxScale`] is relative to the font's `height_unscaled`
/// (`ascent - descent`), *not* `units_per_em`; see [`ScaleFont::scale_factor`].
/// Advances, kerning, ascent and descent all live in that coordinate system, so
/// the outline must use the same factor. Deriving it from `units_per_em` instead
/// would draw glyphs at a size that disagrees with the advances positioning them
/// whenever the two differ (common in CJK fonts), so neighbours overlap.
fn build_glyph_path(
    font: &FontArc,
    glyph_id: ab_glyph::GlyphId,
    scale: PxScale,
) -> Option<tiny_skia::Path> {
    let outline = font.outline(glyph_id)?;
    let factor = font.as_scaled(scale).scale_factor();

    let mut pb = tiny_skia::PathBuilder::new();
    // ab_glyph's coordinate system grows upwards, tiny-skia's grows downwards.
    let map_p = |p: ab_glyph::Point| (p.x * factor.horizontal, -p.y * factor.vertical);

    let mut current_pen: Option<ab_glyph::Point> = None;
    for curve in outline.curves {
        let (start, end) = match curve {
            ab_glyph::OutlineCurve::Line(p1, p2) => (p1, p2),
            ab_glyph::OutlineCurve::Quad(p1, _, p3) => (p1, p3),
            ab_glyph::OutlineCurve::Cubic(p1, _, _, p4) => (p1, p4),
        };

        // A change of contour start point means the previous contour ended;
        // within one contour curves are chained start-to-end.
        if current_pen != Some(start) {
            let (sx, sy) = map_p(start);
            pb.move_to(sx, sy);
        }

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
        current_pen = Some(end);
    }

    pb.finish()
}

/// Box-downsamples a premultiplied RGBA pixmap by an integer factor.
///
/// tiny-skia stores pixels premultiplied, so a straight per-channel average is
/// correct and no un-premultiplication step is needed. If the source
/// dimensions are not exact multiples of `factor`, any trailing rows/columns
/// that do not fill a complete block are ignored.
///
/// Returns `None` if the factor is zero or the source is too small to produce
/// a non-empty result. A factor of `0` or `1` returns a clone/unchanged copy.
pub(crate) fn downsample_premultiplied(src: &Pixmap, factor: u32) -> Option<Pixmap> {
    if factor <= 1 {
        return Some(src.clone());
    }

    let src_width = src.width();
    let src_height = src.height();
    let dst_width = src_width / factor;
    let dst_height = src_height / factor;
    if dst_width == 0 || dst_height == 0 {
        return None;
    }

    let mut dst = Pixmap::new(dst_width, dst_height)?;
    let src_pixels = src.data();
    let samples = factor * factor;
    let half = samples / 2;

    // `src` and `dst` are distinct allocations; every index below stays within
    // their respective dimensions.
    let dst_pixels = dst.data_mut();
    for y in 0..dst_height {
        for x in 0..dst_width {
            let mut acc = [0u32; 4];
            for dy in 0..factor {
                let src_y = y * factor + dy;
                let row = (src_y * src_width) as usize * 4;
                for dx in 0..factor {
                    let src_x = x * factor + dx;
                    let i = row + src_x as usize * 4;
                    acc[0] += src_pixels[i] as u32;
                    acc[1] += src_pixels[i + 1] as u32;
                    acc[2] += src_pixels[i + 2] as u32;
                    acc[3] += src_pixels[i + 3] as u32;
                }
            }
            let o = (y * dst_width + x) as usize * 4;
            dst_pixels[o] = ((acc[0] + half) / samples) as u8;
            dst_pixels[o + 1] = ((acc[1] + half) / samples) as u8;
            dst_pixels[o + 2] = ((acc[2] + half) / samples) as u8;
            dst_pixels[o + 3] = ((acc[3] + half) / samples) as u8;
        }
    }

    Some(dst)
}

#[cfg(test)]
mod downsample_tests {
    use super::*;

    #[test]
    fn solid_color_survives_downsample() {
        let mut src = Pixmap::new(4, 4).unwrap();
        src.fill(Color::from_rgba8(10, 20, 30, 255));

        let dst = downsample_premultiplied(&src, 2).unwrap();
        assert_eq!((dst.width(), dst.height()), (2, 2));
        for px in dst.data().chunks_exact(4) {
            assert_eq!(px, [10, 20, 30, 255]);
        }
    }

    #[test]
    fn factor_one_is_a_passthrough() {
        let mut src = Pixmap::new(3, 2).unwrap();
        src.fill(Color::from_rgba8(1, 2, 3, 255));
        let dst = downsample_premultiplied(&src, 1).unwrap();
        assert_eq!(dst.data(), src.data());
    }

    #[test]
    fn too_small_source_returns_none() {
        let src = Pixmap::new(1, 1).unwrap();
        assert!(downsample_premultiplied(&src, 2).is_none());
    }
}

#[cfg(test)]
mod glyph_path_tests {
    use super::*;

    /// The path handed to tiny-skia must match the outline ab_glyph produces via
    /// `outline_glyph`, i.e. scaled by `scale_factor()` (which is relative to
    /// `height_unscaled`), not by `units_per_em`.
    ///
    /// Regression guard for glyphs being rendered larger than the advances that
    /// position them, which made `2.0` read as `20` (the `2` covered the dot).
    /// Uses the bundled Inter font, so it is deterministic on every machine.
    #[test]
    fn path_matches_ab_glyph_scaled_outline() {
        let stack = crate::core::font::get_raster_fonts(
            "sans-serif",
            crate::core::font::FontRequest::NORMAL,
        )
        .unwrap();
        let font = stack.primary();
        let size = 100.0_f32;
        let scale = PxScale::from(size);

        for c in ['M', 'W', 'g', '2', '.', '0'] {
            let gid = font.glyph_id(c);
            let path = build_glyph_path(font, gid, scale)
                .unwrap_or_else(|| panic!("no glyph path for {c:?}"));
            let b = path.bounds();

            let glyph = gid.with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));
            let outline = font.outline_glyph(glyph).expect("outline_glyph");
            let pb = outline.px_bounds();

            // px_bounds is floor/ceil rounded to whole pixels, so allow ~1px.
            let tol = 1.5;
            assert!(
                (b.left() - pb.min.x).abs() <= tol,
                "{c:?} left {} vs {}",
                b.left(),
                pb.min.x
            );
            assert!(
                (b.right() - pb.max.x).abs() <= tol,
                "{c:?} right {} vs {}",
                b.right(),
                pb.max.x
            );
            assert!(
                (b.top() - pb.min.y).abs() <= tol,
                "{c:?} top {} vs {}",
                b.top(),
                pb.min.y
            );
            assert!(
                (b.bottom() - pb.max.y).abs() <= tol,
                "{c:?} bottom {} vs {}",
                b.bottom(),
                pb.max.y
            );
        }
    }
}
