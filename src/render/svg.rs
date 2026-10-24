//! The SVG document: root element, accessibility metadata and `<defs>`.
//!
//! `SvgBackend` only knows how to write fragments. This module owns the
//! *document* concerns:
//!
//! * the `<svg>` root and its `viewBox`,
//! * the accessible name and description (`role` + `<title>`/`<desc>` plus
//!   `aria-labelledby`/`aria-describedby`),
//! * the single `<defs>` block (clip paths, gradients, user-supplied patterns),
//! * the namespace that makes every `id` unique on a page that inlines several
//!   charts.
//!
//! It is deliberately separate from [`Theme`](crate::theme::Theme): a theme
//! describes how a chart *looks*, an `SvgOptions` describes how the rendered
//! document is *serialised*. The same chart can be exported as a standalone
//! asset and as an accessible inline graphic without cloning its theme.

use crate::core::semantic::A11y;
use crate::error::ChartonError;
use crate::render::backend::svg::{SvgBackend, write_escaped};
use std::fmt::Write;

/// Options for the SVG backend.
///
/// ```
/// use charton::core::semantic::A11y;
/// use charton::render::svg::SvgOptions;
///
/// let options = SvgOptions::default()
///     .with_a11y(A11y::Document)
///     .with_id_prefix("sales")
///     .with_defs(r#"<pattern id="hatch" width="4" height="4">…</pattern>"#);
/// assert!(options.classes);
/// ```
///
/// The struct is `#[non_exhaustive]` with builder methods, so new options can be
/// added without breaking callers.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SvgOptions {
    /// Emit semantic classes (`charton-mark`, `charton-tick-label`, …).
    ///
    /// On by default: classes do not change the rendered look, they only let a
    /// page stylesheet address parts of the chart by role. Turn them off for a
    /// byte-minimal asset.
    pub classes: bool,
    /// How much accessibility metadata to emit. Defaults to [`A11y::Image`].
    pub a11y: A11y,
    /// Namespace for every generated `id`.
    ///
    /// Defaults to a hash of the drawing itself, so inlining several charts in
    /// one page cannot let their `url(#…)` references collide. Set it explicitly
    /// when you want readable, stable ids (e.g. for CSS or JavaScript that
    /// references a gradient by name), or when two *identical* charts share a
    /// page and the duplicate ids must still be avoided.
    pub id_prefix: Option<String>,
    /// Raw SVG fragments to append to the document's `<defs>` block.
    ///
    /// Use it for patterns, filters, markers or your own gradients that a
    /// stylesheet references with `url(#id)`. The fragments are inserted
    /// verbatim: their well-formedness and id uniqueness are the caller's
    /// responsibility. Remember to namespace the ids you define here with
    /// [`SvgOptions::id_prefix`] if several charts share a page.
    pub custom_defs: Vec<String>,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            classes: true,
            a11y: A11y::default(),
            id_prefix: None,
            custom_defs: Vec::new(),
        }
    }
}

impl SvgOptions {
    /// Default options: semantic classes, image-level accessibility, derived ids.
    pub fn new() -> Self {
        Self::default()
    }

    /// Turns semantic classes on or off.
    pub const fn with_classes(mut self, classes: bool) -> Self {
        self.classes = classes;
        self
    }

    /// Emits no semantic classes (smaller output, not stylable by role).
    pub const fn without_classes(self) -> Self {
        self.with_classes(false)
    }

    /// Sets the accessibility mode.
    pub const fn with_a11y(mut self, a11y: A11y) -> Self {
        self.a11y = a11y;
        self
    }

    /// Sets the namespace used for every generated `id`.
    pub fn with_id_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.id_prefix = Some(prefix.into());
        self
    }

    /// Appends a raw fragment to the document's `<defs>` block.
    pub fn with_defs(mut self, defs: impl Into<String>) -> Self {
        self.custom_defs.push(defs.into());
        self
    }
}

/// Everything the SVG document needs that the backend does not know about.
pub(crate) struct SvgDocument<'a> {
    pub width: u32,
    pub height: u32,
    /// The chart title, used for `<title>` and as the accessible name.
    pub title: Option<&'a str>,
    /// The chart description, used for `<desc>`.
    pub description: Option<&'a str>,
    pub options: &'a SvgOptions,
}

impl SvgDocument<'_> {
    /// Renders the document, with `draw` producing the body.
    ///
    /// The body is written first, into its own buffer, for two reasons: every
    /// definition the backend collected can then be emitted in a single
    /// `<defs>` block ahead of it, and the default id namespace can be derived
    /// from the finished drawing.
    pub(crate) fn render<F>(self, draw: F) -> Result<String, ChartonError>
    where
        F: FnOnce(&mut SvgBackend<'_>) -> Result<(), ChartonError>,
    {
        // Until the namespace is known, ids are written with a placeholder that
        // cannot occur in real content.
        let generated = self.options.id_prefix.is_none();
        let prefix = match self.options.id_prefix.as_deref() {
            Some(prefix) => sanitize_id(prefix),
            None => ID_PLACEHOLDER.to_string(),
        };

        let mut body = String::new();
        let defs = {
            let mut backend =
                SvgBackend::new(&mut body, &prefix, self.options.classes, self.options.a11y);
            draw(&mut backend)?;
            backend.into_defs()
        };

        // The namespace is a hash of the drawing: two identical drawings produce
        // identical documents (so output is reproducible and cacheable), and two
        // different drawings can never share an id (so several charts inlined
        // into one page cannot steal each other's clip paths or gradients).
        let (body, defs, prefix) = if generated {
            let prefix = format!(
                "charton-{:08x}",
                (fnv1a(body.as_bytes()) ^ fnv1a(defs.as_bytes())) as u32
            );
            (
                body.replace(ID_PLACEHOLDER, &prefix),
                defs.replace(ID_PLACEHOLDER, &prefix),
                prefix,
            )
        } else {
            (body, defs, prefix)
        };

        self.assemble(&prefix, &defs, &body)
    }

    /// Writes the root element, the accessibility metadata, the single `<defs>`
    /// block and the body.
    fn assemble(self, prefix: &str, defs: &str, body: &str) -> Result<String, ChartonError> {
        let mut svg = String::with_capacity(body.len() + defs.len() + 512);
        let _ = write!(
            svg,
            r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg""#,
            self.width, self.height, self.width, self.height
        );
        if self.options.classes {
            let _ = svg.write_str(r#" class="charton""#);
        }

        // The accessible name. `role="img"` without a name is worse than no role
        // at all (the graphic becomes an unlabelled node), so it is only emitted
        // when there is something to announce.
        let described = self.accessible_ids(prefix);
        if let Some((name_id, description_id)) = &described {
            let role = match self.options.a11y {
                A11y::Document => "graphics-document",
                _ => "img",
            };
            let _ = write!(
                svg,
                r#" role="{role}" aria-roledescription="chart" aria-labelledby="{name_id}""#
            );
            if let Some(description_id) = description_id {
                let _ = write!(svg, r#" aria-describedby="{description_id}""#);
            }
        }
        svg.push_str(
            ">
",
        );

        if described.is_some() {
            if let Some(title) = self.title {
                let _ = write!(svg, r#"<title id="{prefix}-title">"#);
                write_escaped(&mut svg, title);
                svg.push_str(
                    "</title>
",
                );
            }
            if let Some(description) = self.description {
                let _ = write!(svg, r#"<desc id="{prefix}-desc">"#);
                write_escaped(&mut svg, description);
                svg.push_str(
                    "</desc>
",
                );
            }
        }

        if !defs.is_empty() || !self.options.custom_defs.is_empty() {
            svg.push_str("<defs>");
            for custom in &self.options.custom_defs {
                svg.push_str(custom);
            }
            svg.push_str(defs);
            svg.push_str(
                "</defs>
",
            );
        }

        svg.push_str(body);
        svg.push_str("</svg>");
        Ok(svg)
    }

    /// The ids used as the accessible name and description.
    ///
    /// SVG maps `<title>` to the *name* and `<desc>` to the *description*; the
    /// ARIA attributes repeat that mapping explicitly because support for the
    /// native mapping of inline SVG is uneven. They must stay separate: listing
    /// both in `aria-labelledby` would fold the description into the name and
    /// leave the graphic without one.
    ///
    /// When there is a description but no title, the description becomes the
    /// name — an unnamed `role="img"` would be worse than no role at all.
    fn accessible_ids(&self, prefix: &str) -> Option<(String, Option<String>)> {
        if !self.options.a11y.is_enabled() {
            return None;
        }
        match (self.title.is_some(), self.description.is_some()) {
            (true, has_description) => Some((
                format!("{prefix}-title"),
                has_description.then(|| format!("{prefix}-desc")),
            )),
            (false, true) => Some((format!("{prefix}-desc"), None)),
            (false, false) => None,
        }
    }
}

/// Placeholder written in place of the generated id namespace.
///
/// The `\u{1}` character cannot occur in an id, in text content or in an
/// attribute value, so the substitution performed after rendering cannot
/// rewrite real content by accident.
const ID_PLACEHOLDER: &str = "charton-\u{1}-id-";

/// Trims a string down to characters that are legal inside an XML `Name`.
///
/// A user-supplied prefix ends up in `id="…"` and `url(#…)`, so a stray space
/// or `#` would silently produce a broken document.
fn sanitize_id(prefix: &str) -> String {
    let mut out = String::with_capacity(prefix.len());
    for ch in prefix.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    // An id must start with a letter or '_'.
    if out
        .chars()
        .next()
        .is_none_or(|first| !(first.is_ascii_alphabetic() || first == '_'))
    {
        out.insert_str(0, "charton-");
    }
    out
}

/// FNV-1a: a tiny, deterministic string hash.
///
/// Determinism matters here: the id prefix must be identical for identical
/// charts across processes, or HTTP caching and golden-file tests would see a
/// different document on every run.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinate::Rect;
    use crate::core::layer::RenderBackend;
    use crate::core::semantic::{Group, Role};

    fn render(options: &SvgOptions, title: Option<&str>, desc: Option<&str>) -> String {
        SvgDocument {
            width: 100,
            height: 50,
            title,
            description: desc,
            options,
        }
        .render(|backend| {
            backend.begin_group(&Group::new(Role::Plot).clip(Rect::new(0.0, 0.0, 10.0, 10.0)));
            backend.end_group();
            Ok(())
        })
        .expect("rendering an empty document cannot fail")
    }

    #[test]
    fn root_is_always_well_formed() {
        let svg = render(&SvgOptions::default(), None, None);
        assert!(svg.starts_with("<svg width=\"100\" height=\"50\""));
        assert!(svg.contains(r#"xmlns="http://www.w3.org/2000/svg""#));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn definitions_come_before_the_body() {
        let svg = render(&SvgOptions::default(), None, None);
        let defs = svg.find("<defs>").expect("clip path goes into <defs>");
        let body = svg.find("<g").expect("the group is drawn");
        assert!(defs < body, "definitions must precede their use");
        assert_eq!(svg.matches("<defs>").count(), 1, "only one defs block");
    }

    #[test]
    fn custom_defs_are_emitted_in_the_same_block() {
        let options = SvgOptions::default().with_defs("<pattern id=\"hatch\" />");
        let svg = render(&options, None, None);
        assert_eq!(svg.matches("<defs>").count(), 1);
        assert!(svg.contains(r#"<pattern id="hatch" />"#));
        assert!(svg.contains("clipPath"));
    }

    #[test]
    fn a_title_becomes_an_accessible_name() {
        let svg = render(&SvgOptions::default(), Some("Revenue"), None);
        assert!(svg.contains(r#"role="img""#));
        assert!(svg.contains(r#"aria-roledescription="chart""#));
        assert!(svg.contains("<title id=\"charton-"), "{svg}");
        assert!(svg.contains(">Revenue</title>"));
        // `role="img"` needs a name; without a title the role must be omitted.
        let unnamed = render(&SvgOptions::default(), None, None);
        assert!(!unnamed.contains("role="));
        assert!(!unnamed.contains("<title"));
    }

    /// Pulls the value of one quoted attribute out of a document.
    fn attr_of<'a>(svg: &'a str, name: &str) -> Option<&'a str> {
        svg.split(&format!("{name}=\""))
            .nth(1)
            .and_then(|rest| rest.split('"').next())
    }

    #[test]
    fn the_name_and_the_description_stay_separate() {
        let svg = render(&SvgOptions::default(), Some("T"), Some("D"));

        // The name is the title; the description is announced separately. Listing
        // both in `aria-labelledby` would merge them into one long name.
        let name = attr_of(&svg, "aria-labelledby").expect("aria-labelledby");
        let description = attr_of(&svg, "aria-describedby").expect("aria-describedby");
        assert!(name.ends_with("-title"), "{name}");
        assert!(description.ends_with("-desc"), "{description}");
        assert_ne!(name, description);
        assert!(!svg.contains("aria-hidden"));
        assert!(svg.contains("<desc"));
    }

    #[test]
    fn a_description_without_a_title_becomes_the_name() {
        let svg = render(&SvgOptions::default(), None, Some("Only a description"));
        // `role="img"` must have a name, so the description fills both roles: it
        // is the name, and there is nothing left to describe.
        assert!(svg.contains(r#"role="img""#));
        let name = attr_of(&svg, "aria-labelledby").expect("aria-labelledby");
        assert!(name.ends_with("-desc"), "{name}");
        assert!(!svg.contains("aria-describedby"));
    }

    #[test]
    fn document_mode_hides_decorative_groups() {
        let options = SvgOptions::default().with_a11y(A11y::Document);
        let svg = SvgDocument {
            width: 10,
            height: 10,
            title: Some("T"),
            description: None,
            options: &options,
        }
        .render(|backend| {
            backend.begin_group(&Group::new(Role::Grid));
            backend.end_group();
            Ok(())
        })
        .expect("render");
        assert!(svg.contains(r#"role="graphics-document""#));
        assert!(svg.contains(r#"class="charton-grid""#));
        assert!(svg.contains(r#"aria-hidden="true""#));
    }

    #[test]
    fn a11y_off_emits_no_metadata() {
        let options = SvgOptions::default().with_a11y(A11y::Off);
        let svg = render(&options, Some("T"), Some("D"));
        assert!(!svg.contains("role="));
        assert!(!svg.contains("aria-"));
        assert!(!svg.contains("<title"));
        assert!(!svg.contains("<desc"));
    }

    #[test]
    fn ids_are_namespaced_and_deterministic() {
        let first = render(&SvgOptions::default(), Some("A"), None);
        let second = render(&SvgOptions::default(), Some("A"), None);
        assert_eq!(first, second, "same chart, same document");
        let id = first
            .split("clipPath id=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("clip id");
        assert!(
            id.starts_with("charton-") && id.ends_with("-clip-0"),
            "{id}"
        );
    }

    #[test]
    fn explicit_prefix_wins_and_is_sanitized() {
        let options = SvgOptions::default().with_id_prefix("my chart#1");
        let svg = render(&options, None, None);
        assert!(svg.contains(r#"id="my-chart-1-clip-0""#), "{svg}");
        let numeric = SvgOptions::default().with_id_prefix("42");
        let svg = render(&numeric, None, None);
        assert!(svg.contains(r#"id="charton-42-clip-0""#));
    }
}
