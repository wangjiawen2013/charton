//! Font resolution shared by the raster (PNG) and PDF backends.
//!
//! A `font_family` value is a CSS-like stack, for example
//! `"Inter, 'Segoe UI', sans-serif"`. Names are tried left to right and the
//! generic CSS names map onto [`fontdb`]'s generic families, mirroring how a
//! browser resolves `font-family`. The bundled Inter face backs the
//! `sans-serif` generic and is the last-resort fallback.
//!
//! Resolution produces an ordered *candidate chain*: the faces the caller asked
//! for, then a curated system fallback, then the bundled face. Renderers walk
//! that chain per character, so a glyph missing from the requested face is
//! served by the next face that has it instead of becoming a `.notdef` box.
//!
//! Both backends resolve through this module, so they always agree on which
//! face a stack selects.

/// Default CSS-like font stack, shared by the default theme, new text marks and
/// the font fallback chain.
pub(crate) const DEFAULT_FONT_STACK: &str = "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, 'PingFang SC', 'Microsoft YaHei', Ubuntu, Cantarell, 'Noto Sans', sans-serif";

// =============================== Resolution =====================================

#[cfg(any(feature = "png", feature = "pdf"))]
use std::sync::OnceLock;

#[cfg(any(feature = "png", feature = "pdf"))]
use crate::error::ChartonError;
#[cfg(feature = "png")]
use ab_glyph::{Font, GlyphId, PxScale, ScaleFont};
#[cfg(any(feature = "png", feature = "pdf"))]
use ab_glyph::{FontArc, FontVec};
#[cfg(feature = "png")]
use ahash::AHashMap;
#[cfg(any(feature = "png", feature = "pdf"))]
use fontdb::{Database, Family, ID, Query, Stretch, Style, Weight};

/// Process-wide font database: every system font plus the bundled Inter face.
///
/// Building it is expensive (it scans the OS font directories), so it is done
/// once on first use.
#[cfg(any(feature = "png", feature = "pdf"))]
fn db() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = Database::new();
        db.load_system_fonts();
        // The bundled face keeps output readable on machines that have none of
        // the requested families, and gives the `sans-serif` generic a target.
        db.load_font_data(include_bytes!("../../assets/fonts/Inter-Regular.ttf").to_vec());
        db.set_sans_serif_family("Inter");
        db
    })
}

/// Maps a CSS generic family name onto `fontdb`'s generic families; any other
/// name is treated as a concrete family name.
#[cfg(any(feature = "png", feature = "pdf"))]
const fn family(name: &str) -> Family<'_> {
    if name.eq_ignore_ascii_case("sans-serif") {
        Family::SansSerif
    } else if name.eq_ignore_ascii_case("serif") {
        Family::Serif
    } else if name.eq_ignore_ascii_case("monospace") {
        Family::Monospace
    } else if name.eq_ignore_ascii_case("cursive") {
        Family::Cursive
    } else if name.eq_ignore_ascii_case("fantasy") {
        Family::Fantasy
    } else {
        Family::Name(name)
    }
}

/// The non-empty names of a CSS-like family stack, trimmed and unquoted.
#[cfg(any(feature = "png", feature = "pdf"))]
fn family_names(stack: &str) -> impl Iterator<Item = &str> {
    stack
        .split(',')
        .map(|raw| raw.trim().trim_matches(['\'', '"']).trim())
        .filter(|name| !name.is_empty())
}

/// Face properties a text run asks for beyond the family stack.
///
/// `fontdb` matches these against the installed faces, so `"bold"` or
/// `"italic"` text picks the bold/italic member of a family when one exists and
/// the closest available variant otherwise. There is no public stretch knob yet,
/// so `stretch` is always normal; it stays in the request to keep the query
/// complete.
#[cfg(any(feature = "png", feature = "pdf"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FontRequest {
    pub(crate) weight: Weight,
    pub(crate) stretch: Stretch,
    pub(crate) style: Style,
}

#[cfg(any(feature = "png", feature = "pdf"))]
impl FontRequest {
    pub(crate) const NORMAL: FontRequest = FontRequest {
        weight: Weight::NORMAL,
        stretch: Stretch::Normal,
        style: Style::Normal,
    };

    /// Parses the CSS `font-weight` and `font-style` strings carried by a
    /// [`TextConfig`](crate::core::layer::TextConfig). Unrecognised values fall
    /// back to normal, matching CSS.
    pub(crate) fn from_css(weight: &str, style: &str) -> Self {
        FontRequest {
            weight: match weight.trim().to_ascii_lowercase().as_str() {
                "bold" => Weight::BOLD,
                "normal" | "" => Weight::NORMAL,
                other => other.parse::<u16>().map(Weight).unwrap_or(Weight::NORMAL),
            },
            style: match style.trim().to_ascii_lowercase().as_str() {
                "italic" => Style::Italic,
                "oblique" => Style::Oblique,
                _ => Style::Normal,
            },
            ..FontRequest::NORMAL
        }
    }
}

/// Runs a single-family `fontdb` query with the requested face properties.
#[cfg(any(feature = "png", feature = "pdf"))]
fn query(families: &[Family<'_>], request: FontRequest) -> Option<ID> {
    db().query(&Query {
        families,
        weight: request.weight,
        stretch: request.stretch,
        style: request.style,
    })
}

/// The bundled Inter face, always available as the final fallback.
#[cfg(any(feature = "png", feature = "pdf"))]
fn fallback() -> ID {
    query(&[Family::SansSerif], FontRequest::NORMAL).expect("bundled Inter font must be present")
}

#[cfg(any(feature = "png", feature = "pdf"))]
fn push_unique(ids: &mut Vec<ID>, id: ID) {
    if !ids.contains(&id) {
        ids.push(id);
    }
}

/// Resolves a CSS-like family stack to an ordered candidate chain.
///
/// The first element is the primary face (the one whose metrics and baseline
/// the renderer uses). Later elements exist to supply glyphs the earlier faces
/// are missing: a font such as `Inter` covers Latin but not CJK, and CJK fonts
/// sometimes lack symbols like `²`.
///
/// # Errors
/// Returns [`ChartonError::Font`] when none of the named families exist. A
/// stack is allowed to mix available and unavailable names (that is normal CSS
/// behaviour), but if *nothing* the caller asked for can be found, silently
/// using a different font would hide the user's intent, so it is an error.
/// Generic names such as `sans-serif` always resolve through the bundled face.
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) fn resolve_candidates(
    stack: &str,
    request: FontRequest,
) -> Result<Vec<ID>, ChartonError> {
    let mut ids = Vec::new();

    // 1. Faces the caller explicitly asked for, in order. The first match becomes
    //    the primary face.
    for name in family_names(stack) {
        if let Some(id) = query(&[family(name)], request) {
            push_unique(&mut ids, id);
        }
    }
    if ids.is_empty() {
        return Err(ChartonError::Font(format!(
            "no font matched the family stack {stack:?}; install one of the named fonts \
             or use a generic family such as \"sans-serif\""
        )));
    }

    // 2. Curated system fallback, so scripts the requested stack does not cover
    //    still render instead of becoming `.notdef` boxes.
    for name in family_names(DEFAULT_FONT_STACK) {
        if let Some(id) = query(&[family(name)], request) {
            push_unique(&mut ids, id);
        }
    }

    // 3. The bundled face is always available.
    push_unique(&mut ids, fallback());
    Ok(ids)
}

// =============================== Raster (PNG) =====================================

/// One laid-out character: the face that will draw it plus the metrics needed
/// to position it. `kern` is the same-face kerning that precedes this glyph and
/// is applied to the *previous* glyph's advance.
#[cfg(feature = "png")]
pub(crate) struct GlyphPlacement {
    pub(crate) face: usize,
    pub(crate) glyph: GlyphId,
    pub(crate) kern: f32,
    pub(crate) advance: f32,
}

/// A resolved raster font chain. `faces[0]` is the primary face; the rest are
/// fallbacks for characters the earlier faces cannot draw.
#[cfg(feature = "png")]
pub(crate) struct RasterFontStack {
    faces: Vec<FontArc>,
}

#[cfg(feature = "png")]
impl RasterFontStack {
    /// The face whose metrics define the line's ascent/descent.
    pub(crate) fn primary(&self) -> &FontArc {
        &self.faces[0]
    }

    pub(crate) fn face(&self, index: usize) -> &FontArc {
        &self.faces[index]
    }

    /// The first face that has a glyph for `c`, or the primary face when none
    /// does (in which case the renderer draws `.notdef`).
    pub(crate) fn face_for(&self, c: char) -> usize {
        self.faces
            .iter()
            .position(|f| f.glyph_id(c) != GlyphId(0))
            .unwrap_or(0)
    }

    /// Per-character face selection, advance and same-face kerning for `text`.
    ///
    /// Kerning is only meaningful between two glyphs of the same face, so it is
    /// dropped wherever the selected face changes.
    pub(crate) fn layout(&self, text: &str, scale: PxScale) -> Vec<GlyphPlacement> {
        let mut placements = Vec::with_capacity(text.chars().count());
        let mut previous: Option<(usize, GlyphId)> = None;

        for c in text.chars() {
            let face = self.face_for(c);
            let font = &self.faces[face];
            let glyph = font.glyph_id(c);
            let scaled = font.as_scaled(scale);
            let kern = match previous {
                Some((prev_face, prev_glyph)) if prev_face == face => {
                    scaled.kern(prev_glyph, glyph)
                }
                _ => 0.0,
            };
            placements.push(GlyphPlacement {
                face,
                glyph,
                kern,
                advance: scaled.h_advance(glyph),
            });
            previous = Some((face, glyph));
        }

        placements
    }
}

#[cfg(feature = "png")]
use std::sync::{Arc, RwLock};

/// Resolved chains keyed by family stack and then face request, so a stack is
/// only parsed, queried and loaded once per weight/style combination.
#[cfg(feature = "png")]
type RasterStacks = AHashMap<String, AHashMap<FontRequest, Arc<RasterFontStack>>>;

#[cfg(feature = "png")]
static RASTER_STACKS: OnceLock<RwLock<RasterStacks>> = OnceLock::new();

#[cfg(feature = "png")]
fn raster_stacks() -> &'static RwLock<RasterStacks> {
    RASTER_STACKS.get_or_init(|| RwLock::new(AHashMap::new()))
}

/// Loads one face as an `ab_glyph` font, honouring the face index inside a
/// font collection (`.ttc`).
#[cfg(feature = "png")]
fn load_raster_font(id: ID) -> Option<FontArc> {
    db().with_face_data(id, |data, index| {
        FontVec::try_from_vec_and_index(data.to_vec(), index)
            .ok()
            .map(FontArc::new)
    })
    .flatten()
}

/// Resolves a family stack and face request to a cached raster font chain.
///
/// # Errors
/// Propagates the [`ChartonError::Font`] from [`resolve_candidates`], or reports
/// one if the matched faces could not be loaded.
#[cfg(feature = "png")]
pub(crate) fn get_raster_fonts(
    stack: &str,
    request: FontRequest,
) -> Result<Arc<RasterFontStack>, ChartonError> {
    if let Some(by_request) = raster_stacks().read().unwrap().get(stack)
        && let Some(fonts) = by_request.get(&request)
    {
        return Ok(fonts.clone());
    }

    let faces: Vec<FontArc> = resolve_candidates(stack, request)?
        .into_iter()
        .filter_map(load_raster_font)
        .collect();
    if faces.is_empty() {
        return Err(ChartonError::Font(format!(
            "matched fonts for the family stack {stack:?} could not be loaded"
        )));
    }

    let fonts = Arc::new(RasterFontStack { faces });
    raster_stacks()
        .write()
        .unwrap()
        .entry(stack.to_string())
        .or_default()
        .insert(request, fonts.clone());
    Ok(fonts)
}

// =============================== PDF =====================================

/// Builds the krilla and `ab_glyph` handles for one font face.
#[cfg(feature = "pdf")]
fn build_pdf_font(data: Vec<u8>, index: u32) -> Option<(krilla::text::Font, FontArc)> {
    let pdf_font = krilla::text::Font::new(data.clone().into(), index)?;
    let metrics_font = FontArc::new(FontVec::try_from_vec_and_index(data, index).ok()?);
    Some((pdf_font, metrics_font))
}

#[cfg(feature = "pdf")]
fn load_pdf_font(id: ID) -> Option<(krilla::text::Font, FontArc)> {
    db().with_face_data(id, |data, index| build_pdf_font(data.to_vec(), index))
        .flatten()
}

/// Resolves a family stack to an ordered chain of PDF font pairs: a krilla font
/// for embedding text and an `ab_glyph` font for measuring it. The first pair is
/// the primary face; the rest supply missing glyphs.
///
/// # Errors
/// Propagates the [`ChartonError::Font`] from [`resolve_candidates`], or reports
/// one if the matched faces could not be loaded.
#[cfg(feature = "pdf")]
pub(crate) fn resolve_pdf_fonts(
    stack: &str,
    request: FontRequest,
) -> Result<Vec<(krilla::text::Font, FontArc)>, ChartonError> {
    let fonts: Vec<(krilla::text::Font, FontArc)> = resolve_candidates(stack, request)?
        .into_iter()
        .filter_map(load_pdf_font)
        .collect();
    if fonts.is_empty() {
        return Err(ChartonError::Font(format!(
            "matched fonts for the family stack {stack:?} could not be loaded"
        )));
    }
    Ok(fonts)
}

#[cfg(all(test, any(feature = "png", feature = "pdf")))]
mod tests {
    use super::*;

    /// A stack whose first name is missing falls through to the next name rather
    /// than failing as a whole, keeping CSS semantics: an unavailable preference
    /// is skipped, not treated as an error.
    #[test]
    fn stack_falls_through_to_its_next_name() {
        let ids = resolve_candidates("NoSuchFontXYZ123, sans-serif", FontRequest::NORMAL).unwrap();
        assert_eq!(ids.first().copied(), Some(fallback()));
    }

    /// A stack that names nothing available is an error, not a silent fallback.
    #[test]
    fn unknown_family_is_an_error() {
        let err = resolve_candidates("NoSuchFontXYZ123", FontRequest::NORMAL).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("NoSuchFontXYZ123"), "unhelpful: {message}");
    }

    #[test]
    fn quotes_and_whitespace_are_trimmed() {
        let ids = resolve_candidates("  'sans-serif'  ", FontRequest::NORMAL).unwrap();
        assert_eq!(ids.first().copied(), Some(fallback()));
    }

    #[test]
    fn font_request_parses_css_values() {
        let bold_italic = FontRequest::from_css("Bold", "Italic");
        assert_eq!(bold_italic.weight, Weight::BOLD);
        assert_eq!(bold_italic.style, Style::Italic);

        let numeric = FontRequest::from_css("600", "");
        assert_eq!(numeric.weight, Weight(600));
        assert_eq!(numeric.style, Style::Normal);

        // Unrecognised values fall back to normal rather than erroring.
        assert_eq!(FontRequest::from_css("", ""), FontRequest::NORMAL);
        assert_eq!(
            FontRequest::from_css("nonsense", "slanted"),
            FontRequest::NORMAL
        );
    }

    /// The chain starts on the bundled face when nothing else is requested, and
    /// never lists the same face twice.
    #[test]
    fn default_stack_prefers_bundled_inter_without_duplicates() {
        let ids = resolve_candidates(DEFAULT_FONT_STACK, FontRequest::NORMAL).unwrap();
        assert_eq!(ids.first().copied(), Some(fallback()));
        assert_eq!(ids.iter().filter(|&&id| id == fallback()).count(), 1);
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "candidate chain has duplicates");
    }
}
