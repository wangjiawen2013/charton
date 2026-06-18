use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PathTopology, PolygonConfig,
    RectConfig, RenderBackend, TextConfig,
};
use crate::visual::color::SingleColor;
use std::fmt::Write;

/// `SvgBackend` implements the `RenderBackend` trait with a focus on performance.
///
/// Traditional SVG generators often create many temporary `String` objects (e.g., via `format!`).
/// This implementation uses a "Zero-Allocation" streaming strategy: it writes XML fragments
/// and numerical data directly into a pre-allocated `String` buffer.
///
/// This significantly reduces pressure on the system allocator (jemalloc/malloc) when
/// rendering charts with tens of thousands of data points.
pub struct SvgBackend<'a> {
    /// Target buffer where SVG XML content is appended.
    pub buffer: &'a mut String,
}

impl<'a> SvgBackend<'a> {
    /// Creates a new `SvgBackend` wrapped around an external string stream.
    pub const fn new(buffer: &'a mut String) -> Self {
        Self { buffer }
    }

    /// Directly formats a `SingleColor` into the SVG buffer as an `rgba()` string.
    ///
    /// By writing directly to the buffer, we avoid the overhead of creating
    /// intermediate `String` objects for every color application.
    fn write_color(&mut self, color: &SingleColor) {
        if color.is_none() {
            let _ = self.buffer.write_str("none");
        } else {
            let c = color.rgba();
            let _ = write!(
                self.buffer,
                "rgba({},{},{},{:.3})",
                (c[0] * 255.0).round() as u8,
                (c[1] * 255.0).round() as u8,
                (c[2] * 255.0).round() as u8,
                c[3]
            );
        }
    }
}

impl<'a> RenderBackend for SvgBackend<'a> {
    // =========================================================================
    // STATE MACHINE SCOPE IMPLEMENTATION
    // =========================================================================

    fn begin_clip_scope(&mut self, rect: &crate::coordinate::Rect) {
        let id = "plot-clip-area";
        // Define the clipPath inside a structural defs block
        let _ = writeln!(
            self.buffer,
            r#"<defs><clipPath id="{}"><rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" /></clipPath></defs>"#,
            id, rect.x, rect.y, rect.width, rect.height
        );
        // Open a grouping isolation element targeting the clipPath definition
        let _ = writeln!(self.buffer, r#"<g clip-path="url(#{})">"#, id);
    }

    fn end_clip_scope(&mut self) {
        // Terminate the isolated grouping context
        let _ = self.buffer.write_str("</g>\n");
    }

    // =========================================================================
    // 🎨 SHAPE DRAWING METHODS
    // =========================================================================

    fn draw_circle(&mut self, config: CircleConfig) {
        let CircleConfig {
            x,
            y,
            radius,
            fill,
            stroke,
            stroke_width,
            opacity,
        } = config;
        if fill.is_none() && stroke.is_none() {
            return;
        }

        let _ = write!(
            self.buffer,
            r#"<circle cx="{:.3}" cy="{:.3}" r="{:.3}" fill=""#,
            x, y, radius
        );
        self.write_color(&fill);
        let _ = write!(self.buffer, r#"" stroke=""#);
        self.write_color(&stroke);
        let _ = write!(
            self.buffer,
            r#"" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}" />"#,
            stroke_width, opacity, opacity
        );
        let _ = self.buffer.write_str("\n");
    }

    fn draw_rect(&mut self, config: RectConfig) {
        let RectConfig {
            x,
            y,
            width,
            height,
            fill,
            stroke,
            stroke_width,
            opacity,
        } = config;
        if fill.is_none() && stroke.is_none() {
            return;
        }

        let _ = write!(
            self.buffer,
            r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill=""#,
            x, y, width, height
        );
        self.write_color(&fill);
        let _ = write!(self.buffer, r#"" stroke=""#);
        self.write_color(&stroke);
        let _ = write!(
            self.buffer,
            r#"" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}" />"#,
            stroke_width, opacity, 1.0
        );
        let _ = self.buffer.write_str("\n");
    }

    fn draw_path(&mut self, config: PathConfig) {
        let PathConfig {
            points,
            fill,
            stroke,
            stroke_width,
            opacity,
            dash,
            topology,
        } = config;

        if points.is_empty() || (fill.is_none() && stroke.is_none()) {
            return;
        }

        // 1. Write path data (d attribute)
        let _ = self.buffer.write_str(r#"<path d=""#);
        for (i, (px, py)) in points.iter().enumerate() {
            if i == 0 {
                let _ = write!(self.buffer, "M {:.3} {:.3}", px, py);
            } else {
                let _ = write!(self.buffer, " L {:.3} {:.3}", px, py);
            }
        }

        // Close the path automatically for area fills (Complex topology)
        if matches!(topology, PathTopology::Complex) || !fill.is_none() {
            let _ = self.buffer.write_str(" Z");
        }
        let _ = self.buffer.write_str(r#"""#);

        // 2. Write Fill attributes
        let _ = self.buffer.write_str(r#" fill=""#);
        if fill.is_none() {
            let _ = self.buffer.write_str("none\"");
        } else {
            self.write_color(&fill);
            let _ = write!(self.buffer, r#"" fill-opacity="{:.3}""#, opacity);
        }

        // 3. Write Stroke attributes
        let _ = self.buffer.write_str(r#" stroke=""#);
        if stroke.is_none() {
            let _ = self.buffer.write_str("none\"");
        } else {
            self.write_color(&stroke);
            let _ = write!(
                self.buffer,
                r#"" stroke-width="{:.3}" stroke-opacity="{:.3}" stroke-linejoin="round" stroke-linecap="round""#,
                stroke_width, opacity
            );
        }

        // 4. Dash array attributes
        if !dash.is_empty() {
            let dash_str: Vec<String> = dash.iter().map(|d| d.to_string()).collect();
            let _ = write!(self.buffer, r#" stroke-dasharray="{}""#, dash_str.join(","));
        }

        let _ = self.buffer.write_str(r#" />"#);
        let _ = self.buffer.write_str("\n");
    }

    fn draw_polygon(&mut self, config: PolygonConfig) {
        let PolygonConfig {
            points,
            fill,
            stroke,
            stroke_width,
            opacity,
        } = config;
        if points.is_empty() {
            return;
        }

        let _ = self.buffer.write_str(r#"<polygon points=""#);
        for (i, (px, py)) in points.iter().enumerate() {
            let _ = write!(
                self.buffer,
                "{}{:.3},{:.3}",
                if i == 0 { "" } else { " " },
                px,
                py
            );
        }

        let _ = write!(self.buffer, r#"" fill=""#);
        self.write_color(&fill);
        let _ = write!(self.buffer, r#"" stroke=""#);
        self.write_color(&stroke);
        let _ = write!(
            self.buffer,
            r#"" stroke-width="{:.3}" fill-opacity="{:.3}" />"#,
            stroke_width, opacity
        );
        let _ = self.buffer.write_str("\n");
    }

    fn draw_text(&mut self, config: TextConfig) {
        let TextConfig {
            x,
            y,
            text,
            font_size,
            font_family,
            color,
            text_anchor,
            font_weight,
            dominant_baseline,
            opacity,
            angle,
        } = config;

        let _ = write!(
            self.buffer,
            r#"<text x="{:.3}" y="{:.3}" font-size="{:.1}" font-family="{}" fill=""#,
            x, y, font_size, font_family
        );
        self.write_color(&color);
        let _ = write!(
            self.buffer,
            r#"" fill-opacity="{:.3}" text-anchor="{}" font-weight="{}" dominant-baseline="{}""#,
            opacity, text_anchor, font_weight, dominant_baseline
        );
        let _ = write!(
            self.buffer,
            r#" transform="rotate({} {:.3} {:.3})""#,
            angle, x, y
        );
        let _ = self.buffer.write_str(">");

        // Character escaping for XML safety
        self.buffer.push_str(&html_escape::encode_safe(&text));

        let _ = self.buffer.write_str("</text>\n");
    }

    fn draw_line(&mut self, config: LineConfig) {
        let LineConfig {
            x1,
            y1,
            x2,
            y2,
            color,
            width,
            opacity,
            dash,
        } = config;
        let _ = write!(
            self.buffer,
            r#"<line x1="{:.3}" y1="{:.3}" x2="{:.3}" y2="{:.3}" stroke=""#,
            x1, y1, x2, y2
        );
        self.write_color(&color);
        let _ = write!(
            self.buffer,
            r#"" stroke-width="{:.3}" stroke-opacity="{:.3}""#,
            width, opacity
        );

        // Dash line style
        if !dash.is_empty() {
            let dash_str: Vec<String> = dash.iter().map(|d| format!("{:.1}", d)).collect();
            let _ = write!(self.buffer, r#" stroke-dasharray="{}""#, dash_str.join(","));
        }

        let _ = self.buffer.write_str(" />\n");
    }

    fn draw_gradient_rect(&mut self, config: GradientRectConfig) {
        let GradientRectConfig {
            x,
            y,
            width,
            height,
            stops,
            is_vertical,
            id_suffix,
        } = config;
        let (x2, y2) = if is_vertical {
            ("0%", "100%")
        } else {
            ("100%", "0%")
        };

        // Linear gradients require a definition block
        let _ = write!(
            self.buffer,
            r#"<defs><linearGradient id="grad_{}" x1="0%" y1="0%" x2="{}" y2="{}">"#,
            id_suffix, x2, y2
        );
        for (offset, color) in stops {
            let _ = write!(
                self.buffer,
                r#"<stop offset="{:.1}%" stop-color=""#,
                offset * 100.0
            );
            self.write_color(&color);
            let _ = self.buffer.write_str(r#"" />"#);
        }

        // Fixed: standard syntax url(#id) without single quotes for cross-backend safety
        let _ = write!(
            self.buffer,
            r#"</linearGradient></defs><rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="url(#grad_{})" />"#,
            x, y, width, height, id_suffix
        );
        let _ = self.buffer.write_str("\n");
    }
}
