use crate::Precision;
use crate::core::font::{em_scale, line_metrics, run_width, shape};
use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PathTopology, PolygonConfig,
    RectConfig, RenderBackend, TextConfig,
};
use crate::core::semantic::Group;
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
    /// What every open scope must restore when it closes.
    clip_stack: Vec<SavedClip>,
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
            clip_stack: Vec::new(),
        }
    }

    /// Builds the mask for a clip rectangle in device pixels.
    fn build_clip_mask(
        pixmap: &Pixmap,
        transform: &Transform,
        rect: &crate::coordinate::Rect,
    ) -> Option<Mask> {
        let mut mask = Mask::new(pixmap.width(), pixmap.height())?;
        let mut pb = PathBuilder::new();

        // Apply the DPI scale transform to the clipping rectangle bounds
        let skia_rect = SkiaRect::from_xywh(
            (rect.x as f32) * transform.sx,
            (rect.y as f32) * transform.sy,
            (rect.width as f32) * transform.sx,
            (rect.height as f32) * transform.sy,
        )?;
        pb.push_rect(skia_rect);
        let path = pb.finish()?;

        // Fill the mask path with a winding rule to define the clip area
        mask.fill_path(
            &path,
            FillRule::Winding,
            true,                  // Anti-aliasing
            Transform::identity(), // Bounds are already pre-scaled
        );
        Some(mask)
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

/// What closing a scope restores.
enum SavedClip {
    /// The scope installed no clip, so the enclosing mask stays active.
    Keep,
    /// The scope installed a clip; restore the mask that was active before it
    /// (which may itself be none).
    Restore(Option<Mask>),
}

impl<'a> RenderBackend for RasterBackend<'a> {
    // =========================================================================
    // STATE MACHINE SCOPE IMPLEMENTATION
    // =========================================================================

    fn begin_group(&mut self, group: &Group<'_>) {
        let Some(rect) = group.clip else {
            // No clip: keep the enclosing mask and remember that nothing is owed.
            self.clip_stack.push(SavedClip::Keep);
            return;
        };
        let previous = self.clip_mask.take();
        self.clip_mask = Self::build_clip_mask(self.pixmap, &self.transform, &rect);
        self.clip_stack.push(SavedClip::Restore(previous));
    }

    fn end_group(&mut self) {
        // Restore the mask of the enclosing scope (or no mask at the outermost).
        if let Some(SavedClip::Restore(previous)) = self.clip_stack.pop() {
            self.clip_mask = previous;
        }
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
        let faces = stack.faces();

        // 1. Lay the run out in em pixels. `font_size` is an em size, the same
        //    one the SVG and PDF backends use; `shape` picks a face for every
        //    character and returns its advance and kerning. This one layout
        //    drives both the anchor arithmetic and the drawing below.
        let placements = shape(faces, &config.text, config.font_size);
        let width = run_width(&placements);

        let anchor_x = config.x;
        let anchor_y = config.y;

        // 2. Horizontal anchor, matching SVG's `text-anchor`: `middle` and
        //    `end` shift the run by its own width.
        let mut dx = 0.0;
        match config.text_anchor.as_str() {
            "middle" => dx -= width / 2.0,
            "end" => dx -= width,
            _ => {}
        }

        // 3. Vertical anchor, matching SVG's `dominant-baseline`. The default
        //    (alphabetic) baseline sits at `config.y`; `middle` centres the
        //    line box on the anchor, and descent is negative.
        let mut dy = 0.0;
        let metrics = line_metrics(&faces[0], config.font_size);
        match config.dominant_baseline.as_str() {
            "hanging" => dy += metrics.ascent,
            "central" | "middle" => dy += (metrics.ascent + metrics.descent) / 2.0,
            _ => {}
        }

        // 4. Rotation, matching SVG's transform="rotate(angle, x, y)".
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

        // 5. Draw. Every face gets its own em scale, so a glyph supplied by a
        //    fallback face is exactly as large as the primary face's glyphs.
        let scales: Vec<PxScale> = faces
            .iter()
            .map(|face| em_scale(face, config.font_size))
            .collect();

        for placement in &placements {
            current_x += placement.kern;

            if let Some(path) = build_glyph_path(
                &faces[placement.face],
                placement.glyph,
                scales[placement.face],
            ) {
                // Rotation first, then move the glyph to its pen position.
                let glyph_transform = global_transform.pre_translate(current_x, draw_y);
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    glyph_transform,
                    self.clip_mask.as_ref(),
                );
            }

            current_x += placement.advance;
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
            .. // the field name matters only to formats with definitions
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

/// Builds the tiny-skia path for one glyph outline, relative to the glyph's pen
/// position (baseline at `(0, 0)`, y growing downwards).
///
/// `scale` must be the em scale of the face that supplies the glyph, i.e.
/// [`crate::core::font::em_scale`]. The run's advances were measured with the
/// same scale, so outline and advance always agree; using any other scale draws
/// glyphs wider than the space reserved for them, which is how `2.0` once read
/// as `20`.
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
mod font_tests;

#[cfg(test)]
mod glyph_path_tests {
    use super::*;
    use crate::core::font::{FontRequest, em_scale, get_raster_fonts};

    /// The bundled Inter face, resolved through the same chain the renderer
    /// uses, so the tests do not depend on the fonts installed on the machine.
    fn inter() -> FontArc {
        let stack = get_raster_fonts("sans-serif", FontRequest::NORMAL).unwrap();
        stack.faces()[0].clone()
    }

    /// `font_size` is an em size, so one em must be exactly `font_size` pixels
    /// regardless of how tall the face's line box is.
    ///
    /// Inter's line box is ~1.21x its em, so treating `PxScale::from(font_size)`
    /// as the em size used to draw text ~17% too small; a face such as PingFang
    /// SC was off by ~30%.
    #[test]
    fn em_scale_makes_one_em_equal_font_size() {
        let font = inter();
        let font_size = 64.0_f32;
        let scale = em_scale(&font, font_size);
        let upm = font.units_per_em().expect("units_per_em");

        let em_pixels = font.as_scaled(scale).scale_factor().horizontal * upm;
        assert!(
            (em_pixels - font_size).abs() < 0.01,
            "one em should be {font_size}px, got {em_pixels}px"
        );

        // The conversion must not be the identity: ab_glyph's scale is the line
        // box height, which differs from the em size for this face.
        assert!(
            (scale.y - font_size).abs() > 1.0,
            "Inter's line box ({}) should not equal its em size ({font_size})",
            scale.y
        );
    }

    /// The path handed to tiny-skia must match the outline ab_glyph produces for
    /// the same em scale, and the ink must stay inside the advance that positions
    /// it.
    ///
    /// Regression guard for glyphs being rendered larger than their advances,
    /// which made `2.0` read as `20` (the `2` covered the dot).
    #[test]
    fn path_matches_ab_glyph_scaled_outline() {
        let font = inter();
        let font_size = 100.0_f32;
        let scale = em_scale(&font, font_size);
        let scaled = font.as_scaled(scale);

        for c in ['M', 'W', 'g', '2', '.', '0'] {
            let gid = font.glyph_id(c);
            let path = build_glyph_path(&font, gid, scale)
                .unwrap_or_else(|| panic!("no glyph path for {c:?}"));
            let b = path.bounds();

            let glyph = gid.with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));
            let outline = font.outline_glyph(glyph).expect("outline_glyph");
            let pb = outline.px_bounds();

            // px_bounds is floor/ceil rounded to whole pixels, so allow ~1px.
            let tol = 1.5;
            for (actual, expected, edge) in [
                (b.left(), pb.min.x, "left"),
                (b.right(), pb.max.x, "right"),
                (b.top(), pb.min.y, "top"),
                (b.bottom(), pb.max.y, "bottom"),
            ] {
                assert!(
                    (actual - expected).abs() <= tol,
                    "{c:?} {edge} {actual} vs {expected}"
                );
            }

            let advance = scaled.h_advance(gid);
            assert!(
                b.width() <= advance + 1.0,
                "{c:?} ink is {}px wide but its advance is only {advance}px",
                b.width()
            );
        }
    }
}
