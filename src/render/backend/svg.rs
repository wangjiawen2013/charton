use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, PathTopology, PolygonConfig,
    RectConfig, RenderBackend, TextConfig,
};
use crate::core::semantic::{A11y, Element, Group, Role};
use crate::error::ChartonError;
use crate::visual::color::SingleColor;
use std::fmt::Write;

/// Writes `input` into `out`, escaping the characters that are unsafe in XML.
///
/// This applies to *both* text nodes and attribute values, because either can
/// carry user text: a title may contain `<`, and a theme's `font-family` may
/// contain `&` or `"`. Only four characters ever need escaping there — `&`, `<`,
/// `>` and `"` — and every attribute charton writes is quoted with `"`, so `'`
/// and `/` can be written as they are.
pub(crate) fn write_escaped(out: &mut String, input: &str) {
    // Fast path: the vast majority of labels contain nothing to escape.
    if !input
        .bytes()
        .any(|b| matches!(b, b'&' | b'<' | b'>' | b'"'))
    {
        out.push_str(input);
        return;
    }

    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
}

/// `SvgBackend` implements the `RenderBackend` trait with a focus on performance.
///
/// Traditional SVG generators often create many temporary `String` objects (e.g.,
/// via `format!`). This implementation streams XML fragments and numbers straight
/// into a pre-allocated `String`, which is what keeps a chart with tens of
/// thousands of points off the allocator.
///
/// Definitions (`<clipPath>`, `<linearGradient>`) are the exception: they must
/// live in a single `<defs>` block that the document assembly emits *before* the
/// body, so they are accumulated separately.
///
/// # Semantics
///
/// A scope ([`RenderBackend::begin_group`]) becomes `<g class="charton-…">` with
/// its identity attributes and, when the accessibility mode asks for it, the
/// `aria-*` attributes; a clip rectangle on the scope becomes a `clip-path`
/// reference plus one `<clipPath>` in `defs`. A per-element annotation
/// ([`RenderBackend::begin_element`]) writes its value **on the primitive**,
/// because a rule that matches a parent container cannot override the
/// presentation attributes the primitive already carries.
pub struct SvgBackend<'a> {
    /// Target buffer where the SVG body is appended.
    body: &'a mut String,
    /// Accumulated `<clipPath>` / `<linearGradient>` definitions.
    defs: String,
    /// Namespace for every emitted `id`, so several charts can be inlined into
    /// one HTML document without their `url(#…)` references colliding.
    prefix: &'a str,
    /// Whether semantic classes are emitted.
    classes: bool,
    /// How much accessibility metadata to emit.
    a11y: A11y,
    clip_id: usize,
    gradient_id: usize,
    /// The value of the open element scope, reused across elements so that
    /// per-bar identity does not allocate.
    element_value: String,
    /// The series of the open element scope, for marks drawn as a flat stream.
    element_series: String,
    in_element: bool,
}

impl<'a> SvgBackend<'a> {
    /// Creates a new `SvgBackend` writing into `body`.
    ///
    /// `prefix` namespaces every `id`; it must satisfy the XML `Name`
    /// production (letters, digits, `-`, `_`, `.`).
    pub const fn new(body: &'a mut String, prefix: &'a str, classes: bool, a11y: A11y) -> Self {
        Self {
            body,
            defs: String::new(),
            prefix,
            classes,
            a11y,
            clip_id: 0,
            gradient_id: 0,
            element_value: String::new(),
            element_series: String::new(),
            in_element: false,
        }
    }

    /// Consumes the backend and returns the collected definition block.
    pub fn into_defs(self) -> String {
        self.defs
    }

    /// Directly formats a `SingleColor` into the SVG buffer as an `rgba()` string.
    ///
    /// By writing directly to the buffer, we avoid the overhead of creating
    /// intermediate `String` objects for every color application.
    fn write_color(&mut self, color: &SingleColor) {
        if color.is_none() {
            let _ = self.body.write_str("none");
        } else {
            let c = color.rgba();
            let _ = write!(
                self.body,
                "rgba({},{},{},{:.3})",
                (c[0] * 255.0).round() as u8,
                (c[1] * 255.0).round() as u8,
                (c[2] * 255.0).round() as u8,
                c[3]
            );
        }
    }

    /// Appends ` value` escaped, for an attribute.
    fn attr(&mut self, value: &str) {
        write_escaped(self.body, value);
    }

    /// Writes `data-value="…"` when an element scope is open.
    fn write_element_value(&mut self) {
        if !self.in_element {
            return;
        }
        let _ = self.body.write_str(r#" data-value=""#);
        write_escaped(self.body, &self.element_value);
        self.body.push('"');
        if !self.element_series.is_empty() {
            let _ = self.body.write_str(r#" data-series=""#);
            write_escaped(self.body, &self.element_series);
            self.body.push('"');
        }
    }
}

impl RenderBackend for SvgBackend<'_> {
    // =========================================================================
    // SEMANTIC SCOPES
    // =========================================================================

    fn begin_group(&mut self, group: &Group<'_>) {
        // A clip is a document-level definition plus a reference on the group.
        let clip = group.clip.map(|rect| {
            let id = self.clip_id;
            self.clip_id += 1;
            let _ = writeln!(
                self.defs,
                r#"<clipPath id="{}-clip-{}"><rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" /></clipPath>"#,
                self.prefix, id, rect.x, rect.y, rect.width, rect.height
            );
            id
        });

        let _ = self.body.write_str("<g");
        if self.classes {
            // The class list is built first and closed once, so every token
            // ends up inside the attribute.
            let _ = self.body.write_str(r#" class=""#);
            self.body.push_str(group.role.class());
            if let Some(qualifier) = group.role.qualifier() {
                let _ = write!(self.body, " {qualifier}");
            }
            if matches!(group.role, Role::Mark) {
                if let Some(layer) = group.layer {
                    let _ = write!(self.body, " charton-mark-{layer}");
                }
                if let Some(mark) = group.mark.filter(|mark| !mark.is_empty()) {
                    let _ = self.body.write_str(" charton-mark-");
                    self.attr(mark);
                }
            }
            self.body.push('"');
        }

        if let Role::Axis(kind) = group.role {
            let _ = write!(self.body, r#" data-axis="{}""#, kind.data_value());
        }
        if let Some(layer) = group.layer {
            let _ = write!(self.body, r#" data-layer="{layer}""#);
        }
        if let Some(mark) = group.mark.filter(|mark| !mark.is_empty()) {
            let _ = self.body.write_str(r#" data-mark=""#);
            self.attr(mark);
            self.body.push('"');
        }
        if let Some(series) = group.series {
            let _ = self.body.write_str(r#" data-series=""#);
            self.attr(series);
            self.body.push('"');
        }
        if self.a11y.hides_decorative() {
            if group.role.is_decorative() {
                let _ = self.body.write_str(r#" aria-hidden="true""#);
            }
            if matches!(group.role, Role::Mark) {
                let _ = self.body.write_str(r#" role="graphics-object""#);
            }
            if let Some(label) = group.label {
                let _ = self.body.write_str(r#" aria-label=""#);
                self.attr(label);
                self.body.push('"');
            }
        }
        if let Some(id) = clip {
            let _ = write!(
                self.body,
                r#" clip-path="url(#{}-clip-{})""#,
                self.prefix, id
            );
        }
        let _ = self.body.write_str(">\n");
    }

    fn end_group(&mut self) {
        let _ = self.body.write_str("</g>\n");
    }

    fn begin_element(&mut self, element: Element<'_>) {
        // Reuse the buffers: a per-bar value must not allocate.
        self.element_value.clear();
        self.element_value.push_str(element.value);
        self.element_series.clear();
        if let Some(series) = element.series {
            self.element_series.push_str(series);
        }
        self.in_element = true;
    }

    fn end_element(&mut self) {
        self.in_element = false;
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
            self.body,
            r#"<circle cx="{:.3}" cy="{:.3}" r="{:.3}" fill=""#,
            x, y, radius
        );
        self.write_color(&fill);
        let _ = self.body.write_str(r#"" stroke=""#);
        self.write_color(&stroke);
        let _ = write!(
            self.body,
            r#"" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}""#,
            stroke_width, opacity, opacity
        );
        self.write_element_value();
        let _ = self.body.write_str(" />\n");
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
            self.body,
            r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill=""#,
            x, y, width, height
        );
        self.write_color(&fill);
        let _ = self.body.write_str(r#"" stroke=""#);
        self.write_color(&stroke);
        let _ = write!(
            self.body,
            r#"" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}""#,
            stroke_width, opacity, 1.0
        );
        self.write_element_value();
        let _ = self.body.write_str(" />\n");
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
        let _ = self.body.write_str(r#"<path d=""#);
        for (index, (x, y)) in points.iter().enumerate() {
            if index == 0 {
                let _ = write!(self.body, "M {x:.3} {y:.3}");
            } else {
                let _ = write!(self.body, " L {x:.3} {y:.3}");
            }
        }

        // Close the path automatically for area fills (Complex topology)
        if matches!(topology, PathTopology::Complex) || !fill.is_none() {
            let _ = self.body.write_str(" Z");
        }
        let _ = self.body.write_str("\"");

        // 2. Write Fill attributes
        let _ = self.body.write_str(r#" fill=""#);
        if fill.is_none() {
            let _ = self.body.write_str("none\"");
        } else {
            self.write_color(&fill);
            let _ = write!(self.body, r#"" fill-opacity="{:.3}""#, opacity);
        }

        // 3. Write Stroke attributes
        let _ = self.body.write_str(r#" stroke=""#);
        if stroke.is_none() {
            let _ = self.body.write_str("none\"");
        } else {
            self.write_color(&stroke);
            let _ = write!(
                self.body,
                r#"" stroke-width="{:.3}" stroke-opacity="{:.3}" stroke-linejoin="round" stroke-linecap="round""#,
                stroke_width, opacity
            );
        }

        // 4. Dash array attributes
        if !dash.is_empty() {
            let _ = self.body.write_str(r#" stroke-dasharray=""#);
            for (index, value) in dash.iter().enumerate() {
                if index > 0 {
                    self.body.push(',');
                }
                let _ = write!(self.body, "{value}");
            }
            self.body.push('"');
        }

        self.write_element_value();
        let _ = self.body.write_str(" />\n");
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

        let _ = self.body.write_str(r#"<polygon points=""#);
        for (index, (x, y)) in points.iter().enumerate() {
            let _ = write!(
                self.body,
                "{}{:.3},{:.3}",
                if index == 0 { "" } else { " " },
                x,
                y
            );
        }

        let _ = self.body.write_str(r#"" fill=""#);
        self.write_color(&fill);
        let _ = self.body.write_str(r#"" stroke=""#);
        self.write_color(&stroke);
        let _ = write!(
            self.body,
            r#"" stroke-width="{:.3}" fill-opacity="{:.3}""#,
            stroke_width, opacity
        );
        self.write_element_value();
        let _ = self.body.write_str(" />\n");
    }

    fn draw_text(&mut self, config: TextConfig) -> Result<(), ChartonError> {
        let TextConfig {
            x,
            y,
            text,
            font_size,
            font_family,
            color,
            text_anchor,
            font_weight,
            font_style,
            dominant_baseline,
            opacity,
            angle,
        } = config;

        let _ = write!(
            self.body,
            r#"<text x="{:.3}" y="{:.3}" font-size="{:.1}" font-family=""#,
            x, y, font_size
        );
        self.attr(&font_family);
        let _ = self.body.write_str(r#"" fill=""#);
        self.write_color(&color);
        let _ = self.body.write_str(r#"" fill-opacity=""#);
        let _ = write!(
            self.body,
            r#"{opacity:.3}" text-anchor="{text_anchor}" font-weight="{font_weight}" font-style="{font_style}" dominant-baseline="{dominant_baseline}""#
        );
        let _ = write!(
            self.body,
            r#" transform="rotate({} {:.3} {:.3})""#,
            angle, x, y
        );
        self.write_element_value();
        self.body.push('>');

        // Character escaping for XML safety
        write_escaped(self.body, &text);

        let _ = self.body.write_str("</text>\n");
        Ok(())
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
            self.body,
            r#"<line x1="{:.3}" y1="{:.3}" x2="{:.3}" y2="{:.3}" stroke=""#,
            x1, y1, x2, y2
        );
        self.write_color(&color);
        let _ = write!(
            self.body,
            r#"" stroke-width="{:.3}" stroke-opacity="{:.3}""#,
            width, opacity
        );

        // Dash line style
        if !dash.is_empty() {
            let _ = self.body.write_str(r#" stroke-dasharray=""#);
            for (index, value) in dash.iter().enumerate() {
                if index > 0 {
                    self.body.push(',');
                }
                let _ = write!(self.body, "{value:.1}");
            }
            self.body.push('"');
        }

        self.write_element_value();
        let _ = self.body.write_str(" />\n");
    }

    fn draw_gradient_rect(&mut self, config: GradientRectConfig) {
        let GradientRectConfig {
            x,
            y,
            width,
            height,
            stops,
            is_vertical,
            field,
        } = config;
        let (x2, y2) = if is_vertical {
            ("0%", "100%")
        } else {
            ("100%", "0%")
        };

        // The gradient is a definition, so it goes into the shared defs block
        // with a namespaced id; the drawn rect only references it.
        let id = self.gradient_id;
        self.gradient_id += 1;
        let _ = write!(
            self.defs,
            r#"<linearGradient id="{}-grad-{}" x1="0%" y1="0%" x2="{}" y2="{}""#,
            self.prefix, id, x2, y2
        );
        let _ = write!(self.defs, r#" data-field=""#);
        write_escaped(&mut self.defs, &field);
        self.defs.push_str(r#"">"#);
        for (offset, color) in stops {
            let _ = write!(
                self.defs,
                r#"<stop offset="{:.1}%" stop-color=""#,
                offset * 100.0
            );
            if color.is_none() {
                self.defs.push_str("none");
            } else {
                let c = color.rgba();
                let _ = write!(
                    self.defs,
                    "rgba({},{},{},{:.3})",
                    (c[0] * 255.0).round() as u8,
                    (c[1] * 255.0).round() as u8,
                    (c[2] * 255.0).round() as u8,
                    c[3]
                );
            }
            self.defs.push_str(r#"" />"#);
        }
        self.defs.push_str("</linearGradient>");

        let _ = write!(
            self.body,
            r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="url(#{}-grad-{})" />"#,
            x, y, width, height, self.prefix, id
        );
        self.body.push('\n');
    }
}
