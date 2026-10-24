//! The PDF document: page setup, optional tagging, and serialisation.
//!
//! `PdfBackend` only knows how to draw. This module owns the *document*
//! concerns, mirroring [`render::svg`](crate::render::svg): the page, the
//! accessibility mode and the structure tree that makes the result a tagged
//! PDF.
//!
//! PDF tagging is the counterpart of the SVG's `role`/`aria-*` metadata:
//! decorative scopes (background, grid, frame) become `/Artifact`s, the chart
//! body becomes a `/Figure` with an `/Alt` text, and the axes and legend become
//! paragraphs, so assistive technology announces a chart instead of a pile of
//! unlabelled vector paths.

use crate::core::semantic::A11y;
use crate::error::ChartonError;
use crate::render::backend::pdf::{PdfBackend, PdfTagging};
use krilla::Document;
use krilla::SerializeSettings;
use krilla::geom::Size;
use krilla::page::PageSettings;

/// Options for the PDF backend.
///
/// The struct is `#[non_exhaustive]` with builder methods, so new options can be
/// added without breaking callers.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct PdfOptions {
    /// How much accessibility metadata to emit.
    ///
    /// * [`A11y::Off`] writes a plain PDF: tagging is disabled, so the document
    ///   claims nothing it cannot back up.
    /// * [`A11y::Image`] and [`A11y::Document`] write a tagged PDF with one
    ///   `/Figure` carrying the chart's title and description. PDF has no
    ///   equivalent of "navigable graphics", so both modes produce the same
    ///   structure; keeping one enum makes the export API uniform across
    ///   formats.
    pub a11y: A11y,
    /// Write the PDF in a human-inspectable form (larger file, uncompressed
    /// content). Intended for debugging tests and tooling, not for delivery.
    pub pretty: bool,
}

impl PdfOptions {
    /// Default options: a tagged PDF, compressed.
    pub fn new() -> Self {
        Self {
            a11y: A11y::default(),
            pretty: false,
        }
    }

    /// Sets the accessibility mode.
    pub const fn with_a11y(mut self, a11y: A11y) -> Self {
        self.a11y = a11y;
        self
    }

    /// Writes an inspectable, uncompressed document.
    pub const fn pretty(mut self) -> Self {
        self.pretty = true;
        self
    }
}

/// Combines the chart's title and description into one `/Alt` text.
///
/// PDF has a single alternative text per figure, so both are joined; the
/// description alone is better than nothing when there is no title.
fn alt_text(title: Option<&str>, description: Option<&str>) -> Option<String> {
    match (title, description) {
        (Some(title), Some(description)) => Some(format!(
            "{title}

{description}"
        )),
        (Some(title), None) => Some(title.to_string()),
        (None, Some(description)) => Some(description.to_string()),
        (None, None) => None,
    }
}

/// Everything the PDF document needs that the backend does not know about.
pub(crate) struct PdfDocument<'a> {
    pub width: u32,
    pub height: u32,
    /// The chart title, used as the first half of the figure's `/Alt` text.
    pub title: Option<&'a str>,
    /// The chart description, appended to the `/Alt` text.
    pub description: Option<&'a str>,
    pub options: &'a PdfOptions,
}

impl PdfDocument<'_> {
    /// Renders the document, with `draw` producing the page content.
    pub(crate) fn render<F>(self, draw: F) -> Result<Vec<u8>, ChartonError>
    where
        F: FnOnce(&mut PdfBackend<'_, '_>) -> Result<(), ChartonError>,
    {
        let page_size = Size::from_wh(self.width as f32, self.height as f32)
            .ok_or_else(|| ChartonError::Render("Invalid chart dimensions for PDF".to_string()))?;

        let settings = SerializeSettings {
            enable_tagging: self.options.a11y.is_enabled(),
            pretty: self.options.pretty,
            // An inspectable document should be inspectable all the way down:
            // the marked-content operators (/Artifact, /P, …) live in the page
            // stream.
            compress_content_streams: !self.options.pretty,
            ..SerializeSettings::default()
        };

        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(PageSettings::new(page_size));
        let mut surface = page.surface();

        let tag_tree = {
            let tagging = self.options.a11y.is_enabled().then(|| PdfTagging {
                alt_text: alt_text(self.title, self.description),
            });
            let mut backend = PdfBackend::new(&mut surface, tagging);

            draw(&mut backend)?;
            backend.into_tag_tree()
        };

        surface.finish();
        page.finish();
        if let Some(tree) = tag_tree {
            document.set_tag_tree(tree);
        }
        document
            .finish()
            .map_err(|e| ChartonError::Render(format!("PDF generation error: {e:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinate::Rect;
    use crate::core::layer::RenderBackend;
    use crate::core::semantic::{Group, Role};

    fn render(options: &PdfOptions, alt: Option<&str>) -> Vec<u8> {
        PdfDocument {
            width: 100,
            height: 50,
            title: alt,
            description: None,
            options,
        }
        .render(|backend| {
            backend.begin_group(&Group::new(Role::Background));
            backend.end_group();
            backend.begin_group(&Group::new(Role::Plot).clip(Rect::new(0.0, 0.0, 10.0, 10.0)));
            backend.end_group();
            Ok(())
        })
        .expect("rendering an empty page cannot fail")
    }

    /// PDF dictionaries are written as plain objects, so the structure tree can
    /// be inspected without decompressing content streams.
    fn text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    #[test]
    fn a_tagged_document_declares_marked_content() {
        let pdf = text(&render(&PdfOptions::default().pretty(), Some("Revenue")));
        assert!(pdf.starts_with("%PDF-"), "not a PDF");
        assert!(pdf.contains("/StructTreeRoot"), "no structure tree");
        assert!(pdf.contains("/Figure"), "the chart body is not a figure");
        assert!(
            pdf.contains("/Artifact"),
            "the background is not an artifact"
        );
        assert!(pdf.contains("Revenue"), "the figure has no alt text");
    }

    #[test]
    fn tagging_off_writes_a_plain_document() {
        let pdf = text(&render(
            &PdfOptions::default().with_a11y(A11y::Off).pretty(),
            Some("Revenue"),
        ));
        assert!(pdf.starts_with("%PDF-"));
        assert!(!pdf.contains("/StructTreeRoot"));
        assert!(!pdf.contains("/Artifact"));
    }

    #[test]
    fn a_chart_without_text_still_gets_a_figure() {
        let pdf = text(&render(&PdfOptions::default(), None));
        assert!(pdf.contains("/Figure"));
    }
}
