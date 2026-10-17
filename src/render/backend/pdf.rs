use crate::Precision;
use crate::core::font::{FontRequest, line_metrics, run_width, shape};
use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PathTopology, PolygonConfig,
    RectConfig, RenderBackend, TextConfig,
};
use crate::error::ChartonError;
use crate::visual::color::SingleColor;
use ab_glyph::FontArc;
use ahash::AHashMap;
use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Point, Rect, Transform};
use krilla::num::NormalizedF32;
use krilla::paint::{
    Fill, FillRule, LineCap, LineJoin, LinearGradient, SpreadMethod, Stop, Stroke, StrokeDash,
};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};
use std::sync::Arc;

/// Bézier constant that approximates a quarter circle.
const KAPPA: f32 = 0.552_284_8;

/// One family's ordered font fallback chain: a krilla font for embedding text
/// and an `ab_glyph` font for measuring it. The first pair is the primary face.
type FontChain = Arc<Vec<(Font, FontArc)>>;

/// PDF rendering backend built on top of `krilla`.
///
/// The chart is drawn directly into the PDF page using the same coordinates as
/// the other backends: the origin is the top-left corner and y grows downwards.
/// krilla maps that onto the PDF page when the document is written.
pub struct PdfBackend<'a, 'p> {
    /// The page surface all drawing operations are sent to.
    surface: &'a mut Surface<'p>,
    /// Fonts already resolved for this document, keyed by family name and then
    /// face request. Each entry is the ordered fallback chain for that stack.
    fonts: AHashMap<String, AHashMap<FontRequest, FontChain>>,
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

    /// Returns the resolved font chain for a family and face request, resolving
    /// and caching on first use. The chain's first entry is the primary face.
    fn font(&mut self, family: &str, request: FontRequest) -> Result<FontChain, ChartonError> {
        if let Some(by_request) = self.fonts.get(family)
            && let Some(entry) = by_request.get(&request)
        {
            return Ok(entry.clone());
        }
        let resolved = Arc::new(crate::core::font::resolve_pdf_fonts(family, request)?);
        self.fonts
            .entry(family.to_string())
            .or_default()
            .insert(request, resolved.clone());
        Ok(resolved)
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

    fn draw_text(&mut self, config: TextConfig) -> Result<(), ChartonError> {
        if config.color.is_none() || config.text.is_empty() {
            return Ok(());
        }
        let request = FontRequest::from_css(&config.font_weight, &config.font_style);
        let chain = self.font(&config.font_family, request)?;

        // Shape with the same em-based layout as the raster backend, so both
        // draw the text at the same size and spacing.
        let faces: Vec<FontArc> = chain.iter().map(|(_, metrics)| metrics.clone()).collect();
        let placements = shape(&faces, &config.text, config.font_size);
        let text_width = run_width(&placements);

        // Horizontal anchor, matching SVG's `text-anchor`.
        let start_x = match config.text_anchor.as_str() {
            "middle" => config.x - text_width / 2.0,
            "end" => config.x - text_width,
            _ => config.x,
        };

        // Vertical anchor, matching SVG's `dominant-baseline`.
        let metrics = line_metrics(&faces[0], config.font_size);
        let baseline_y = match config.dominant_baseline.as_str() {
            "hanging" => config.y + metrics.ascent,
            "central" | "middle" => config.y + (metrics.ascent + metrics.descent) / 2.0,
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

        // krilla takes advances in em units and multiplies them by the font
        // size, so the pixel advances are converted back here.
        let to_em = |px: f32| px / config.font_size.max(f32::EPSILON);

        // The byte range of each character, in the order `shape` produced them;
        // krilla needs it to subset the embedded font.
        let byte_ranges: Vec<std::ops::Range<usize>> = config
            .text
            .char_indices()
            .map(|(start, c)| start..start + c.len_utf8())
            .collect();

        // krilla draws one font per `draw_glyphs` call, so consecutive
        // characters sharing a face become a single run.
        let mut run_start = 0usize;
        let mut run_x = start_x;
        while run_start < placements.len() {
            let face = placements[run_start].face;
            let mut run_end = run_start + 1;
            while run_end < placements.len() && placements[run_end].face == face {
                run_end += 1;
            }
            let run = &placements[run_start..run_end];

            let mut glyphs: Vec<KrillaGlyph> = Vec::with_capacity(run.len());
            for (i, p) in run.iter().enumerate() {
                // `kern` belongs to the gap before this glyph, i.e. it shortens
                // the advance of the glyph drawn just before it.
                if i > 0
                    && let Some(last) = glyphs.last_mut()
                {
                    last.x_advance += to_em(p.kern);
                }
                glyphs.push(KrillaGlyph::new(
                    GlyphId::new(p.glyph.0 as u32),
                    to_em(p.advance),
                    0.0,
                    0.0,
                    0.0,
                    byte_ranges[run_start + i].clone(),
                    None,
                ));
            }

            self.surface.draw_glyphs(
                Point::from_xy(run_x, baseline_y),
                &glyphs,
                chain[face].0.clone(),
                &config.text,
                config.font_size,
                false,
            );

            run_x += run.iter().map(|p| p.kern + p.advance).sum::<f32>();
            run_start = run_end;
        }

        if rotated {
            self.surface.pop();
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
