use crate::Precision;
use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PathTopology, PolygonConfig,
    RectConfig, RenderBackend, TextConfig,
};
use crate::visual::color::SingleColor;
use ab_glyph::{Font as AbFont, FontArc, PxScale, ScaleFont};
use ahash::AHashMap;
use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Point, Rect, Transform};
use krilla::num::NormalizedF32;
use krilla::paint::{
    Fill, FillRule, LineCap, LineJoin, LinearGradient, SpreadMethod, Stop, Stroke, StrokeDash,
};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};

/// Bézier constant that approximates a quarter circle.
const KAPPA: f32 = 0.552_284_8;

/// PDF rendering backend built on top of `krilla`.
///
/// The chart is drawn directly into the PDF page using the same coordinates as
/// the other backends: the origin is the top-left corner and y grows downwards.
/// krilla maps that onto the PDF page when the document is written.
pub struct PdfBackend<'a, 'p> {
    /// The page surface all drawing operations are sent to.
    surface: &'a mut Surface<'p>,
    /// Fonts already resolved for this document, keyed by family name.
    fonts: AHashMap<String, Option<(Font, FontArc)>>,
    /// Number of clip regions currently pushed onto the surface.
    clip_depth: usize,
}

impl<'a, 'p> PdfBackend<'a, 'p> {
    /// Creates a backend that draws onto the given page surface.
    pub fn new(surface: &'a mut Surface<'p>) -> Self {
        Self {
            surface,
            fonts: AHashMap::new(),
            clip_depth: 0,
        }
    }

    /// Clamps an opacity to the 0..=1 range that krilla expects.
    fn opacity(value: f32) -> NormalizedF32 {
        NormalizedF32::new(value.clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ONE)
    }

    /// Converts a chart color to an RGB triple.
    fn rgb(color: &SingleColor) -> rgb::Color {
        let c = color.rgba();
        rgb::Color::new(
            (c[0] * 255.0).round() as u8,
            (c[1] * 255.0).round() as u8,
            (c[2] * 255.0).round() as u8,
        )
    }

    /// Builds a solid fill, or `None` when the color is "none".
    fn fill(color: &SingleColor, opacity: Precision) -> Option<Fill> {
        if color.is_none() {
            return None;
        }
        let alpha = color.rgba()[3] * opacity;
        Some(Fill {
            paint: Self::rgb(color).into(),
            opacity: Self::opacity(alpha),
            rule: FillRule::NonZero,
        })
    }

    /// Builds a stroke, or `None` when the color is "none" or the width is zero.
    fn stroke(
        color: &SingleColor,
        width: Precision,
        opacity: Precision,
        line_cap: LineCap,
        line_join: LineJoin,
        dash: &[Precision],
    ) -> Option<Stroke> {
        if color.is_none() || width <= 0.0 {
            return None;
        }
        let alpha = color.rgba()[3] * opacity;
        Some(Stroke {
            paint: Self::rgb(color).into(),
            width,
            miter_limit: 10.0,
            line_cap,
            line_join,
            opacity: Self::opacity(alpha),
            dash: if dash.is_empty() {
                None
            } else {
                Some(StrokeDash {
                    array: dash.to_vec(),
                    offset: 0.0,
                })
            },
        })
    }

    /// Sets the paint state used by the next path or text draw.
    fn set_paint(&mut self, fill: Option<Fill>, stroke: Option<Stroke>) {
        self.surface.set_fill(fill);
        self.surface.set_stroke(stroke);
    }

    /// Builds a rectangle path.
    fn rect_path(x: f32, y: f32, width: f32, height: f32) -> Option<Path> {
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        let rect = Rect::from_xywh(x, y, width, height)?;
        let mut builder = PathBuilder::new();
        builder.push_rect(rect);
        builder.finish()
    }

    /// Builds a path from a point sequence, optionally closing the contour.
    fn line_path(points: &[(Precision, Precision)], close: bool) -> Option<Path> {
        if points.is_empty() {
            return None;
        }
        let mut builder = PathBuilder::new();
        builder.move_to(points[0].0, points[0].1);
        for &(x, y) in &points[1..] {
            builder.line_to(x, y);
        }
        if close {
            builder.close();
        }
        builder.finish()
    }

    /// Builds a circle from four cubic curves, which is how most vector
    /// formats approximate one.
    fn circle_path(cx: f32, cy: f32, radius: f32) -> Option<Path> {
        if radius <= 0.0 {
            return None;
        }
        let k = KAPPA * radius;
        let mut builder = PathBuilder::new();
        builder.move_to(cx + radius, cy);
        builder.cubic_to(cx + radius, cy + k, cx + k, cy + radius, cx, cy + radius);
        builder.cubic_to(cx - k, cy + radius, cx - radius, cy + k, cx - radius, cy);
        builder.cubic_to(cx - radius, cy - k, cx - k, cy - radius, cx, cy - radius);
        builder.cubic_to(cx + k, cy - radius, cx + radius, cy - k, cx + radius, cy);
        builder.close();
        builder.finish()
    }

    /// Returns the resolved fonts for a family, resolving and caching on first use.
    fn font(&mut self, family: &str) -> Option<(Font, FontArc)> {
        if let Some(entry) = self.fonts.get(family) {
            return entry.clone();
        }
        let resolved = crate::core::utils::resolve_pdf_font(family);
        self.fonts.insert(family.to_string(), resolved.clone());
        resolved
    }
}

impl RenderBackend for PdfBackend<'_, '_> {
    fn begin_clip_scope(&mut self, rect: &crate::coordinate::Rect) {
        if let Some(path) = Self::rect_path(
            rect.x as f32,
            rect.y as f32,
            rect.width as f32,
            rect.height as f32,
        ) {
            self.surface.push_clip_path(&path, &FillRule::NonZero);
            self.clip_depth += 1;
        }
    }

    fn end_clip_scope(&mut self) {
        if self.clip_depth > 0 {
            self.surface.pop();
            self.clip_depth -= 1;
        }
    }

    fn draw_circle(&mut self, config: CircleConfig) {
        let fill = Self::fill(&config.fill, config.opacity);
        let stroke = Self::stroke(
            &config.stroke,
            config.stroke_width,
            config.opacity,
            LineCap::Butt,
            LineJoin::Miter,
            &[],
        );
        if fill.is_none() && stroke.is_none() {
            return;
        }
        self.set_paint(fill, stroke);
        if let Some(path) = Self::circle_path(config.x, config.y, config.radius) {
            self.surface.draw_path(&path);
        }
    }

    fn draw_rect(&mut self, config: RectConfig) {
        let fill = Self::fill(&config.fill, config.opacity);
        // Rect strokes are drawn fully opaque, matching the other backends.
        let stroke = Self::stroke(
            &config.stroke,
            config.stroke_width,
            1.0,
            LineCap::Butt,
            LineJoin::Miter,
            &[],
        );
        if fill.is_none() && stroke.is_none() {
            return;
        }
        self.set_paint(fill, stroke);
        if let Some(path) = Self::rect_path(config.x, config.y, config.width, config.height) {
            self.surface.draw_path(&path);
        }
    }

    fn draw_line(&mut self, config: LineConfig) {
        let stroke = Self::stroke(
            &config.color,
            config.width,
            config.opacity,
            LineCap::Butt,
            LineJoin::Miter,
            &config.dash,
        );
        if stroke.is_none() {
            return;
        }
        self.set_paint(None, stroke);
        let points = [(config.x1, config.y1), (config.x2, config.y2)];
        if let Some(path) = Self::line_path(&points, false) {
            self.surface.draw_path(&path);
        }
    }

    fn draw_path(&mut self, config: PathConfig) {
        if config.points.is_empty() || (config.fill.is_none() && config.stroke.is_none()) {
            return;
        }

        let closed = matches!(config.topology, PathTopology::Complex) || !config.fill.is_none();
        let fill = Self::fill(&config.fill, config.opacity);
        let stroke = Self::stroke(
            &config.stroke,
            config.stroke_width,
            config.opacity,
            LineCap::Round,
            LineJoin::Round,
            &config.dash,
        );

        self.set_paint(fill, stroke);
        if let Some(path) = Self::line_path(&config.points, closed) {
            self.surface.draw_path(&path);
        }
    }

    fn draw_polygon(&mut self, config: PolygonConfig) {
        if config.points.is_empty() {
            return;
        }

        let fill = Self::fill(&config.fill, config.opacity);
        let stroke = Self::stroke(
            &config.stroke,
            config.stroke_width,
            1.0,
            LineCap::Butt,
            LineJoin::Miter,
            &[],
        );

        self.set_paint(fill, stroke);
        if let Some(path) = Self::line_path(&config.points, true) {
            self.surface.draw_path(&path);
        }
    }

    fn draw_text(&mut self, config: TextConfig) {
        if config.color.is_none() || config.text.is_empty() {
            return;
        }
        let Some((pdf_font, metrics_font)) = self.font(&config.font_family) else {
            return;
        };

        let scale = PxScale::from(config.font_size);
        let scaled = metrics_font.as_scaled(scale);
        let units_per_em = metrics_font.units_per_em().unwrap_or(1000.0).max(1.0);

        // Lay the text out character by character. Advances are normalized by the
        // font size because krilla scales them back up when drawing.
        let mut glyphs: Vec<KrillaGlyph> = Vec::new();
        let mut byte_offset = 0usize;
        let mut previous: Option<ab_glyph::GlyphId> = None;
        let mut text_width = 0.0f32;

        for ch in config.text.chars() {
            let glyph_id = metrics_font.glyph_id(ch);
            let advance = metrics_font.h_advance_unscaled(glyph_id) / units_per_em;

            // Kerning tightens the gap before this glyph, which means it shortens
            // the advance of the glyph that was laid out just before it.
            if let Some(prev) = previous {
                let kern = metrics_font.kern_unscaled(prev, glyph_id) / units_per_em;
                if let Some(last) = glyphs.last_mut() {
                    last.x_advance += kern;
                }
                text_width += kern * config.font_size;
            }

            let start = byte_offset;
            byte_offset += ch.len_utf8();

            glyphs.push(KrillaGlyph::new(
                GlyphId::new(glyph_id.0 as u32),
                advance,
                0.0,
                0.0,
                0.0,
                start..byte_offset,
                None,
            ));
            text_width += advance * config.font_size;
            previous = Some(glyph_id);
        }

        // Horizontal placement based on the requested text anchor.
        let start_x = match config.text_anchor.as_str() {
            "middle" => config.x - text_width / 2.0,
            "end" => config.x - text_width,
            _ => config.x,
        };

        // Vertical placement based on the requested baseline.
        let ascent = scaled.ascent();
        let descent = scaled.descent();
        let baseline_y = match config.dominant_baseline.as_str() {
            "hanging" => config.y + ascent,
            "central" | "middle" => config.y + (ascent + descent) / 2.0,
            _ => config.y,
        };

        self.set_paint(Self::fill(&config.color, config.opacity), None);

        let rotated = config.angle != 0.0;
        if rotated {
            self.surface.push_transform(&Transform::from_rotate_at(
                config.angle,
                config.x,
                config.y,
            ));
        }

        self.surface.draw_glyphs(
            Point::from_xy(start_x, baseline_y),
            &glyphs,
            pdf_font,
            &config.text,
            config.font_size,
            false,
        );

        if rotated {
            self.surface.pop();
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
            ..
        } = config;

        let color_stops: Vec<Stop> = stops
            .into_iter()
            .filter_map(|(offset, color)| {
                if color.is_none() {
                    return None;
                }
                Some(Stop {
                    offset: Self::opacity(offset),
                    color: Self::rgb(&color).into(),
                    opacity: Self::opacity(color.rgba()[3]),
                })
            })
            .collect();

        let Some(path) = Self::rect_path(x, y, width, height) else {
            return;
        };

        if color_stops.is_empty() {
            return;
        }

        // A single stop carries no direction, so paint it as a solid color.
        if color_stops.len() == 1 {
            self.set_paint(
                Some(Fill {
                    paint: color_stops[0].color.clone().into(),
                    opacity: color_stops[0].opacity,
                    rule: FillRule::NonZero,
                }),
                None,
            );
            self.surface.draw_path(&path);
            return;
        }

        let (x1, y1, x2, y2) = if is_vertical {
            (x, y, x, y + height)
        } else {
            (x, y, x + width, y)
        };

        let gradient = LinearGradient {
            x1,
            y1,
            x2,
            y2,
            transform: Transform::identity(),
            spread_method: SpreadMethod::Pad,
            stops: color_stops,
            anti_alias: true,
        };

        self.set_paint(
            Some(Fill {
                paint: gradient.into(),
                opacity: NormalizedF32::ONE,
                rule: FillRule::NonZero,
            }),
            None,
        );
        self.surface.draw_path(&path);
    }
}
