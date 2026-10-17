//! Regression tests for the raster text pipeline.
//!
//! These mirror the four silent-failure reports in the font issue, plus the
//! cross-backend text-size contract:
//!
//! 1. the default theme's CSS family stack must not collapse to `.notdef`;
//! 2. a family that matches nothing must be an error, not a silent fallback;
//! 3. a missing glyph must be served by a later face in the chain (per-glyph
//!    fallback), including symbols a CJK face happens to lack (`²`);
//! 4. a glyph outline must never be drawn wider than the advance that positions
//!    it, which is what made `2.5` read as `25`;
//! 5. `font_size` is an em size, so PNG text must be the same size as SVG/PDF.
//!
//! They live inside the crate so they can render straight to a `Pixmap` and
//! inspect pixels, and so they reuse the optional `ab_glyph` / `fontdb` deps
//! that the `png` feature already enables — no extra test dependencies.
//!
//! No macOS is required. Fonts are picked by *coverage*, not by name: a face
//! that has `销` plays the role of PingFang SC / Hiragino Sans GB, one that has
//! `销` but lacks `²` plays Hiragino Sans GB in report 3b, one that has `A` but
//! no CJK plays Helvetica in report 3a, and a face whose ascent/descent does not
//! match its `units_per_em` plays PingFang SC in reports 4 and 5. Cases whose
//! required fonts are not installed are skipped with a message. On macOS the
//! exact faces from the report are used as well (see `reported_macos_cases`).
//!
//! Run with:
//!
//! ```text
//! cargo test --features png --lib font_tests -- --nocapture
//! ```

use std::path::Path;

use super::*;

use crate::alt;
use crate::chart::Chart;
use crate::core::composite::LayeredChart;
use crate::core::conversion::IntoLayered;
use crate::core::data::Dataset;
use crate::error::ChartonError;

/// A character no real font maps, used to render the font's `.notdef` glyph.
const NOTDEF: char = '\u{10FFFD}';

/// Control title used to locate the title line: it has an ascender (`M`) and a
/// descender (`gy`), so the band covers the full line box of any script.
const CONTROL_TITLE: &str = "Mgy";

/// The exact skeleton from the issue, rendered straight to pixels.
fn render(title: &str, family: Option<&str>) -> Result<Pixmap, ChartonError> {
    let ds = Dataset::new()
        .with_column("x", vec![1.0, 2.0, 3.0])?
        .with_column("y", vec![2.0, 3.0, 5.0])?;
    let lc: LayeredChart = Chart::build(ds)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_title(title)
        .with_size(400, 200);
    let lc = match family {
        Some(f) => lc.configure_theme(|t| {
            t.with_title_family(f)
                .with_label_family(f)
                .with_tick_label_family(f)
                .with_legend_label_family(f)
        }),
        None => lc,
    };
    lc.render_pixmap()
}

/// The issue's skeleton with a large title on a taller canvas, so the drawn em
/// size can be measured in whole pixels.
fn render_sized(title: &str, family: Option<&str>, title_size: f32) -> Pixmap {
    let ds = Dataset::new()
        .with_column("x", vec![1.0, 2.0, 3.0])
        .unwrap()
        .with_column("y", vec![2.0, 3.0, 5.0])
        .unwrap();
    let lc: LayeredChart = Chart::build(ds)
        .unwrap()
        .mark_point()
        .unwrap()
        .encode((alt::x("x"), alt::y("y")))
        .unwrap()
        .with_title(title)
        .with_size(600, 320);
    let lc = lc.configure_theme(|t| {
        let t = t.with_title_size(f64::from(title_size));
        match family {
            Some(f) => t
                .with_title_family(f)
                .with_label_family(f)
                .with_tick_label_family(f)
                .with_legend_label_family(f),
            None => t,
        }
    });
    lc.render_pixmap().expect("chart should render")
}

// ============================== Pixel helpers =================================

struct Ink {
    width: usize,
    height: usize,
    /// Any appreciably non-paper pixel: used for silhouettes, so the faint
    /// antialiased edge of a `.notdef` box still belongs to the glyph.
    mask: Vec<bool>,
    /// Core ink only: used to split glyphs, so antialiasing halos do not bridge
    /// two characters that are in fact far enough apart.
    strong: Vec<bool>,
}

impl Ink {
    fn from_pixmap(pixmap: &Pixmap) -> Self {
        let width = pixmap.width() as usize;
        let height = pixmap.height() as usize;
        let mut mask = Vec::with_capacity(width * height);
        let mut strong = Vec::with_capacity(width * height);
        for px in pixmap.pixels() {
            // The paper is opaque white and glyphs are drawn over it, so the
            // premultiplied channels are directly comparable to the threshold.
            let (r, g, b, a) = (px.red(), px.green(), px.blue(), px.alpha());
            let luma = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000;
            mask.push(a > 64 && luma < 250);
            strong.push(a > 64 && luma < 150);
        }
        Ink {
            width,
            height,
            mask,
            strong,
        }
    }
    fn get(&self, x: usize, y: usize) -> bool {
        self.mask[y * self.width + x]
    }

    fn strong_at(&self, x: usize, y: usize) -> bool {
        self.strong[y * self.width + x]
    }

    fn row_has_ink(&self, y: usize) -> bool {
        (0..self.width).any(|x| self.get(x, y))
    }

    /// The raw ink bitmap of rows `[y0, y1)`, without cropping or resampling.
    fn window_mask(&self, y0: usize, y1: usize) -> Vec<bool> {
        let mut out = Vec::with_capacity((y1 - y0) * self.width);
        for y in y0..y1 {
            for x in 0..self.width {
                out.push(self.get(x, y));
            }
        }
        out
    }

    /// Horizontal bands of ink, split where there are three or more blank rows.
    ///
    /// The first band is the title and the second starts the plot area. The
    /// threshold is small because a descender (`y` in the control title) can
    /// come within a few rows of the plot; a single text line never contains
    /// three blank rows in a row.
    fn bands(&self) -> Vec<(usize, usize)> {
        let mut bands = Vec::new();
        let mut y = 0;
        while y < self.height {
            if !self.row_has_ink(y) {
                y += 1;
                continue;
            }
            let start = y;
            let mut end = y + 1;
            let mut blank_run = 0;
            for yy in y..self.height {
                if self.row_has_ink(yy) {
                    end = yy + 1;
                    blank_run = 0;
                } else if blank_run == 2 {
                    break;
                } else {
                    blank_run += 1;
                }
            }
            bands.push((start, end));
            y = end;
        }
        bands
    }

    /// The rows that belong to the title: everything above the plot area.
    ///
    /// Taking the plot's top edge from the control render keeps the window
    /// correct even when the tested title is blank, which is itself a failure
    /// mode, and makes the window tall enough for glyphs that overflow the
    /// control's own ink (a `.notdef` box is taller than an `M`).
    fn title_window(&self) -> (usize, usize) {
        let plot_top = self
            .bands()
            .get(1)
            .map(|&(top, _)| top.saturating_sub(1))
            .unwrap_or(self.height);
        (0, plot_top)
    }

    /// Whether any pixel in rows `[y0, y1)` is inked.
    fn has_ink_in(&self, y0: usize, y1: usize) -> bool {
        (y0..y1).any(|y| self.row_has_ink(y))
    }

    /// Number of connected ink blobs inside `[y0, y1)` (8-connectivity).
    ///
    /// `"2.5"` is three blobs (`2`, `.`, `5`); when an oversized outline covers
    /// the period the blobs merge and the count drops, which is report 4.
    fn blobs_in(&self, y0: usize, y1: usize) -> usize {
        let width = self.width;
        let mut seen = vec![false; width * self.height];
        let mut blobs = 0;
        for y in y0..y1 {
            for x in 0..width {
                if !self.strong_at(x, y) || seen[y * width + x] {
                    continue;
                }
                blobs += 1;
                let mut stack = vec![(x, y)];
                seen[y * width + x] = true;
                while let Some((x, y)) = stack.pop() {
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if nx < 0 || ny < y0 as i32 || nx >= width as i32 || ny >= y1 as i32 {
                                continue;
                            }
                            let (nx, ny) = (nx as usize, ny as usize);
                            if self.strong_at(nx, ny) && !seen[ny * width + nx] {
                                seen[ny * width + nx] = true;
                                stack.push((nx, ny));
                            }
                        }
                    }
                }
            }
        }
        blobs
    }
}

/// Jaccard overlap of two same-sized bitmaps, in `[0, 1]`.
fn overlap(a: &[bool], b: &[bool]) -> f32 {
    let mut intersection = 0;
    let mut union = 0;
    for (a, b) in a.iter().zip(b) {
        intersection += usize::from(*a && *b);
        union += usize::from(*a || *b);
    }
    intersection as f32 / union.max(1) as f32
}

fn must_render(title: &str, family: Option<&str>) -> Pixmap {
    render(title, family).unwrap_or_else(|e| panic!("rendering {title:?} as {family:?}: {e}"))
}

fn render_ink(title: &str, family: Option<&str>) -> Ink {
    Ink::from_pixmap(&must_render(title, family))
}

fn save(name: &str, pixmap: &Pixmap) {
    let dir = Path::new("target/test-output/font-issue");
    std::fs::create_dir_all(dir).expect("create output dir");
    let png = pixmap.encode_png().expect("encode png");
    std::fs::write(dir.join(format!("{name}.png")), png).expect("write png");
}

/// The rows that belong to the title, located with a control title drawn in the
/// same font so the window stays put even for a blank title.
fn title_band(family: Option<&str>) -> (usize, usize) {
    render_ink(CONTROL_TITLE, family).title_window()
}

/// Detects `.notdef` (tofu) characters by rendering them one at a time and
/// comparing the silhouette to the font's own `.notdef` glyph.
///
/// Characters are rendered alone because some fonts draw `.notdef` boxes that
/// touch their neighbours, so splitting a whole title into characters by blank
/// columns would miss them.
struct Tofu<'a> {
    family: Option<&'a str>,
    y0: usize,
    y1: usize,
    notdef: Vec<bool>,
}

impl<'a> Tofu<'a> {
    fn new(family: Option<&'a str>) -> Self {
        let (y0, y1) = title_band(family);
        // A private-use code point no real font maps renders as `.notdef`.
        let reference = render_ink(&NOTDEF.to_string(), family);
        let notdef = reference.window_mask(y0, y1);
        assert!(notdef.iter().any(|&p| p), "`.notdef` must produce ink");
        Tofu {
            family,
            y0,
            y1,
            notdef,
        }
    }

    /// `true` when `c` has no glyph in the chain and comes out as `.notdef` (or
    /// as nothing at all).
    fn is_tofu(&self, c: char) -> bool {
        let ink = render_ink(&c.to_string(), self.family);
        // A code point with no glyph maps to glyph 0, so the same `.notdef`
        // outline is drawn at the same position: the window bitmaps match apart
        // from sub-pixel antialiasing. A real glyph differs across the whole
        // line, even when it is as box-like as `销`.
        if !ink.has_ink_in(self.y0, self.y1) {
            return true;
        }
        let drawn = ink.window_mask(self.y0, self.y1);
        // A `.notdef` box lands at ~0.88 because its antialiased edges shift by
        // a fraction of a pixel with the advance; real glyphs stay below 0.4.
        overlap(&drawn, &self.notdef) >= 0.7
    }

    /// `true` when any visible character of `text` is a `.notdef` box.
    fn any_in(&self, text: &str) -> bool {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .any(|c| self.is_tofu(c))
    }
}

// ============================ System font probe ================================
//
// The tests pick fonts by measured coverage rather than by platform-specific
// names, so the assertions are about the renderer, not about one machine.

struct Face {
    family: String,
    font: ab_glyph::FontArc,
}

fn system_faces() -> Vec<Face> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut faces = Vec::new();
    for face in db.faces() {
        let Some((family, _)) = face.families.first() else {
            continue;
        };
        let Some(font) = db.with_face_data(face.id, |data, index| {
            ab_glyph::FontVec::try_from_vec_and_index(data.to_vec(), index)
                .ok()
                .map(ab_glyph::FontArc::new)
        }) else {
            continue;
        };
        let Some(font) = font else { continue };
        faces.push(Face {
            family: family.clone(),
            font,
        });
    }
    faces
}

fn covers(face: &Face, c: char) -> bool {
    use ab_glyph::Font;
    face.font.glyph_id(c).0 != 0
}

/// The first installed family that covers every `need` and none of `missing`.
fn pick_family(need: &[char], missing: &[char]) -> Option<String> {
    system_faces()
        .into_iter()
        .find(|face| {
            need.iter().all(|&c| covers(face, c)) && missing.iter().all(|&c| !covers(face, c))
        })
        .map(|face| face.family)
}

/// Well-behaved fonts whose `height_unscaled` disagrees with `units_per_em`,
/// i.e. the same shape of font as the reporter's PingFang SC. All of them are
/// normal text faces, so "2.5" is expected to be three separate ink blobs.
const PINGFANG_ANALOGUES: &[&str] = &[
    // macOS
    "PingFang SC",
    "Hiragino Sans GB",
    "Heiti SC",
    "Songti SC",
    // Windows
    "Microsoft YaHei",
    "Arial Unicode MS",
    "Noto Sans SC",
    "SimSun",
    // Linux
    "Noto Sans CJK SC",
    "Source Han Sans SC",
    "WenQuanYi Zen Hei",
    "Droid Sans Fallback",
];

/// A family whose `height_unscaled` disagrees with `units_per_em`.
///
/// Scaling an outline by `units_per_em` (the pre-fix bug) is only wrong for
/// those faces, which is exactly the PingFang SC / Hiragino situation in report
/// 4. Inter has `height == units_per_em`, so it cannot demonstrate the bug.
fn pick_mismatched_metrics_family() -> Option<String> {
    use ab_glyph::Font;

    let faces = system_faces();
    let mismatch = |face: &Face| {
        if !['2', '.', '5'].iter().all(|&c| covers(face, c)) {
            return 0.0;
        }
        let upm = face.font.units_per_em().unwrap_or(1000.0);
        face.font.height_unscaled() / upm - 1.0
    };

    // Prefer the curated list, so the analogue is a real text face (a script or
    // handwriting font would break the "2.5 is three blobs" expectation for
    // reasons unrelated to scaling).
    for name in PINGFANG_ANALOGUES {
        if let Some(face) = faces.iter().find(|f| f.family.eq_ignore_ascii_case(name))
            && mismatch(face).abs() > 0.25
        {
            return Some(face.family.clone());
        }
    }

    // Otherwise fall back to the closest mismatch above the threshold.
    faces
        .iter()
        .map(|face| (mismatch(face), face))
        .filter(|(m, _)| m.abs() > 0.25)
        .min_by(|(a, _), (b, _)| a.abs().total_cmp(&b.abs()))
        .map(|(_, face)| face.family.clone())
}

fn skip(reason: &str) {
    eprintln!("skipping font test: {reason}");
}

// ================================== Tests =====================================

/// Report 1: the default theme's stack is CSS syntax, not one family name.
///
/// Before the fix the whole `"Inter, -apple-system, …"` string was handed to
/// `fontdb::Family::Name`, matched nothing, and every CJK character became a
/// `.notdef` box.
#[test]
fn default_stack_renders_cjk_instead_of_notdef() {
    if pick_family(&['销'], &[]).is_none() {
        return skip("no installed font covers CJK, so there is nothing to fall back to");
    }
    let png = must_render("销售额趋势", None);
    save("default-stack-cjk", &png);
    assert!(
        !Tofu::new(None).any_in("销售额趋势"),
        "the default font stack rendered CJK as .notdef boxes"
    );
}

/// Report 2: a family that matches nothing must surface as an error.
///
/// Before the fix it silently rendered with the bundled Inter face, so the two
/// outputs were byte-identical and the user's intent vanished.
#[test]
fn unknown_family_is_an_error() {
    let unknown = "NoSuchFontXYZ123";
    let err = render("Hello", Some(unknown)).expect_err("an unknown family must not render");
    match err {
        ChartonError::Font(message) => assert!(
            message.contains(unknown),
            "the error should name the missing family, got: {message}"
        ),
        other => panic!("expected ChartonError::Font, got {other:?}"),
    }

    // The same chart with a usable family still renders.
    let ok = must_render("Hello", None);
    save("unknown-family-baseline", &ok);
}

/// Report 3a: a Latin-only face (Helvetica on the reporter's Mac) has no CJK
/// glyphs, so the glyphs must come from a later face in the chain.
#[test]
fn latin_only_family_falls_back_per_glyph() {
    let Some(latin) = pick_family(&['A', 'z'], &['销']) else {
        return skip("no Latin-only font found");
    };
    if pick_family(&['销'], &[]).is_none() {
        return skip("no installed font covers CJK, so there is nothing to fall back to");
    }

    // Self-check: the detector must still recognise a genuinely missing glyph.
    let tofu = Tofu::new(Some(&latin));
    assert!(
        tofu.is_tofu(NOTDEF),
        "the tofu detector failed on its own reference"
    );

    let png = must_render("销售额", Some(&latin));
    save("latin-only-cjk", &png);
    assert!(
        !tofu.any_in("销售额"),
        "family {latin:?} has no CJK glyph, and the fallback chain did not fill in: \
         the title is .notdef boxes"
    );
}

/// Report 3b: a CJK face can still lack common symbols (`²`, `³`, `µ`).
///
/// Hiragino Sans GB is such a face on macOS; `pick_family` finds whatever the
/// current machine has (SimHei on Windows, for example).
#[test]
fn cjk_family_without_superscript_falls_back() {
    let Some(cjk) = pick_family(&['销'], &['²', '³', 'µ']) else {
        return skip("no installed CJK font lacks ²/³/µ, so report 3b cannot be exercised here");
    };

    let tofu = Tofu::new(Some(&cjk));
    for symbol in ["²", "³", "µ"] {
        let png = must_render(symbol, Some(&cjk));
        save(&format!("cjk-missing-{symbol}"), &png);
        assert!(
            !tofu.is_tofu(symbol.chars().next().unwrap()),
            "family {cjk:?} has no {symbol:?} glyph and the fallback chain did not fill it in"
        );
    }

    // The full title from the report: Chinese must render as well.
    let png = must_render("销售额（m²）", Some(&cjk));
    save("cjk-missing-superscript-title", &png);
    assert!(
        !tofu.any_in("销售额（m²）"),
        "family {cjk:?} rendered the mixed title as .notdef boxes"
    );
}

/// The bundled Inter face, read straight from the repository so the expected
/// metrics do not depend on what the machine has installed.
fn bundled_inter() -> ab_glyph::FontArc {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/fonts/Inter-Regular.ttf"
    );
    let data = std::fs::read(path).expect("bundled Inter should be readable");
    ab_glyph::FontArc::new(ab_glyph::FontVec::try_from_vec(data).expect("valid Inter face"))
}

/// How tall `c` must be, in pixels, when drawn at `font_size` em.
///
/// This restates the contract independently of the renderer: `font_size` is an
/// em size, so one em is `font_size` pixels, and the glyph height is its outline
/// height scaled by `font_size / units_per_em`.
fn expected_ink_height(font: &ab_glyph::FontArc, c: char, font_size: f32) -> f32 {
    use ab_glyph::{Font, PxScale};

    let upm = font.units_per_em().expect("units_per_em");
    let scale = PxScale::from(font_size * font.height_unscaled() / upm);
    let glyph = font
        .glyph_id(c)
        .with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));
    font.outline_glyph(glyph)
        .expect("glyph should have an outline")
        .px_bounds()
        .height()
}

/// Renders `c` as the title at `font_size` and returns the height of its ink.
fn measured_ink_height(family: Option<&str>, c: char, font_size: f32) -> f32 {
    let png = render_sized(&c.to_string(), family, font_size);
    let ink = Ink::from_pixmap(&png);
    let (top, bottom) = *ink
        .bands()
        .first()
        .unwrap_or_else(|| panic!("{c:?} produced no ink"));
    (bottom - top) as f32
}

/// Whether an exact family name is installed, for the macOS reproduction below.
#[cfg(target_os = "macos")]
fn family_installed(name: &str) -> bool {
    system_faces()
        .iter()
        .any(|face| face.family.eq_ignore_ascii_case(name))
}

/// The four reports with the exact font names from the issue.
///
/// Compiled on macOS only: PingFang SC, Hiragino Sans GB and Helvetica are the
/// faces that produced the report, and having those exact faces is what makes
/// this a faithful reproduction rather than an analogue. Every other platform
/// is covered by the coverage-based tests above.
#[cfg(target_os = "macos")]
#[test]
fn reported_macos_cases() {
    // 1. Default theme stack, Chinese title.
    save("macos-1-default-stack", &must_render("销售额趋势", None));
    assert!(
        !Tofu::new(None).any_in("销售额趋势"),
        "report 1: the default stack drew CJK as .notdef boxes"
    );

    // 2. Unknown family must be an error.
    assert!(
        render("Hello", Some("NoSuchFontXYZ123")).is_err(),
        "report 2: an unknown family rendered instead of failing"
    );

    // 3a. Helvetica exists but has no CJK glyphs.
    if family_installed("Helvetica") {
        save(
            "macos-3a-helvetica",
            &must_render("销售额", Some("Helvetica")),
        );
        assert!(
            !Tofu::new(Some("Helvetica")).any_in("销售额"),
            "report 3a: Helvetica fell back to .notdef boxes instead of a CJK face"
        );
    } else {
        skip("Helvetica is not installed");
    }

    // 3b. Hiragino Sans GB has CJK but lacks ²/³/µ.
    if family_installed("Hiragino Sans GB") {
        let hiragino = Tofu::new(Some("Hiragino Sans GB"));
        save(
            "macos-3b-hiragino",
            &must_render("销售额（m²）", Some("Hiragino Sans GB")),
        );
        assert!(
            !hiragino.is_tofu('²'),
            "report 3b: Hiragino Sans GB drew ² as .notdef"
        );
        assert!(
            !hiragino.any_in("销售额（m²）"),
            "report 3b: the mixed title has .notdef boxes"
        );
    } else {
        skip("Hiragino Sans GB is not installed");
    }

    // 4. PingFang SC swallowed the decimal point: "2.5" read as "25".
    if family_installed("PingFang SC") {
        let (y0, y1) = title_band(Some("PingFang SC"));
        let png = must_render("2.5", Some("PingFang SC"));
        save("macos-4-pingfang-decimal", &png);
        let blobs = Ink::from_pixmap(&png).blobs_in(y0, y1);
        assert_eq!(
            blobs, 3,
            "report 4: PingFang SC drew \"2.5\" as {blobs} ink blobs, not 3 (2, ., 5)"
        );
    } else {
        skip("PingFang SC is not installed");
    }
}

/// Report 4: the outline must be drawn at the scale its advances assume.
///
/// Before the fix glyphs were scaled by `units_per_em` instead of ab_glyph's
/// `scale_factor`, so on faces whose `height_unscaled` differs (PingFang SC,
/// Microsoft YaHei, Hiragino) they were drawn wider than their advances; the
/// oversized `2` covered the `.` and `2.5` was read as `25`. A covered period
/// merges the three ink blobs into fewer.
#[test]
fn decimal_point_is_not_swallowed() {
    let Some(family) = pick_mismatched_metrics_family() else {
        return skip("no installed font has height_unscaled != units_per_em");
    };

    let (y0, y1) = title_band(Some(&family));

    let png = must_render("2.5", Some(&family));
    let blobs = Ink::from_pixmap(&png).blobs_in(y0, y1);
    save("decimal-mismatched-metrics", &png);
    assert_eq!(
        blobs, 3,
        "family {family:?}: \"2.5\" should be three ink blobs (2, ., 5), got {blobs}; \
         the decimal point was swallowed by an oversized neighbour"
    );

    // `25` is two blobs, which shows the counter itself is sound.
    let png = must_render("25", Some(&family));
    let blobs = Ink::from_pixmap(&png).blobs_in(y0, y1);
    assert_eq!(
        blobs, 2,
        "family {family:?}: \"25\" should be two ink blobs, got {blobs}"
    );

    // The bundled face has height == units_per_em, so it is a control: the
    // period must survive there in both the old and the new code.
    let (y0, y1) = title_band(None);
    let png = must_render("2.5", None);
    let blobs = Ink::from_pixmap(&png).blobs_in(y0, y1);
    save("decimal-default-stack", &png);
    assert_eq!(
        blobs, 3,
        "default stack: \"2.5\" should be three ink blobs, got {blobs}"
    );
}

/// Text is drawn at the requested **em** size, for the primary face and for a
/// face whose line box is much taller than its em.
///
/// `font_size` used to be fed to `ab_glyph` as its `PxScale`, which is a line-box
/// height, not an em size. Inter's line box is ~1.21x its em and Microsoft YaHei
/// / PingFang SC are ~1.32-1.4x, so PNG text was 17-30% smaller than the same
/// `font_size` in SVG and PDF. A tolerance of 3px absorbs antialiasing, pixel
/// rounding and the difference between a family's regular and bold faces (the
/// title is bold); the old error was 8-12px at this size.
#[test]
fn text_is_drawn_at_the_requested_em_size() {
    let font_size = 64.0;

    let measured = measured_ink_height(None, 'H', font_size);
    let expected = expected_ink_height(&bundled_inter(), 'H', font_size);
    assert!(
        (measured - expected).abs() <= 3.0,
        "default stack: 'H' is {measured}px tall, expected {expected}px at {font_size}px em"
    );

    // A face whose metrics differ from Inter's, so the per-face conversion is
    // exercised rather than only the bundled face.
    let Some(font_family) = pick_mismatched_metrics_family() else {
        return skip("no installed font has height_unscaled != units_per_em");
    };
    let face = system_faces()
        .into_iter()
        .find(|face| face.family == font_family && covers(face, '销'));
    let Some(face) = face else {
        return skip(&format!("{font_family:?} does not cover CJK"));
    };
    let measured = measured_ink_height(Some(&font_family), '销', font_size);
    let expected = expected_ink_height(&face.font, '销', font_size);
    assert!(
        (measured - expected).abs() <= 3.0,
        "{font_family:?}: '销' is {measured}px tall, expected {expected}px at {font_size}px em"
    );
}
