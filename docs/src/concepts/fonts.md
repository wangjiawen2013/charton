# Fonts & Text Rendering

Every chart has text: the title, axis titles, tick labels, legend entries,
facet strip labels, and any `mark_text` annotations. This page explains how a
`font_family` value becomes visible glyphs, how the library behaves when a font
is missing, and what each rendering backend does differently.

The whole subsystem lives in one place, `src/core/font.rs`. Both the raster
(PNG) and PDF backends resolve their fonts through it, so they always agree on
which face a family selects. The SVG and WGPU backends delegate font selection
to an external renderer and therefore follow a different path (see
[Backend Behaviour](#backend-behaviour) below).

## A font family is a CSS-like stack

The `*_family` fields on a `Theme` and
`MarkText::with_font_family` accept a CSS `font-family` style stack:

```text
Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica,
Arial, 'PingFang SC', 'Microsoft YaHei', Ubuntu, Cantarell, 'Noto Sans',
sans-serif
```

That exact string is the default (`DEFAULT_FONT_STACK`). Names are separated by
commas; surrounding whitespace and single or double quotes are trimmed; names
are tried **left to right**, exactly as a browser would. The generic CSS names
map onto the operating system's generic families:

| CSS name     | Meaning                                     |
| ------------ | ------------------------------------------- |
| `sans-serif` | the bundled **Inter** face                  |
| `serif`      | the system serif family                     |
| `monospace`  | the system monospace family                 |
| `cursive`    | the system cursive family                   |
| `fantasy`    | the system fantasy family                   |

Charton bundles one font, **Inter**, for Latin text. It is registered in two
ways, and the two are the same face reached through two routes:

- as a concrete family named `Inter`, so `"Inter"` in a stack matches it;
- as the target of the generic `sans-serif` (via `set_sans_serif_family`), so
  `"sans-serif"` matches it too.

That is why the default stack can list `Inter` first and `sans-serif` last
without doing the same work twice: the candidate chain de-duplicates by face, so
Inter appears once. It also means `sans-serif` is always satisfiable — even on a
machine with no system sans-serif font — so text can never disappear because a
system has no fonts at all.

## Resolution produces a candidate chain

Resolving a stack does not return a single font. It returns an ordered
**candidate chain**:

1. every face named by the stack that exists, in order (the first one is the
   *primary* face);
2. a curated set of system fallbacks (`FALLBACK_FONT_STACK`), so scripts the
   requested fonts do not cover still render;
3. the bundled Inter face.

The primary face defines the line box (`ascent`/`descent`) and so the baseline.
The remaining faces exist to supply glyphs the primary lacks. Renderers walk the
chain **per character** and draw each character with the first face that
actually contains it. Kerning is only applied between two adjacent characters
that use the same face.

This is what makes mixed-script text work reliably:

```rust
use charton::prelude::*;

fn sales_chart(ds: Dataset) -> LayeredChart {
    Chart::build(ds)
        .unwrap()
        .mark_point()
        .unwrap()
        .encode((alt::x("x"), alt::y("y")))
        .unwrap()
        .with_title("销售额 (m²) — 2024") // Latin, CJK and a symbol
        .configure_theme(|t| {
            t.with_title_family("Helvetica") // primary: Latin + ², no CJK
        })
}
```

Here `Helvetica` is the primary face. `销` and `售` are not in Helvetica, so
they are served by the next face in the chain that has them (for example
`PingFang SC` on macOS or `Microsoft YaHei` on Windows); `m` and `²` stay on
Helvetica.

The fallback list is **curated**, not a scan of every installed font. It covers
CJK and common symbols such as `²`, `µ`, `°`, `℃` and `Ω`, and it lists the CJK
families of each supported platform (`PingFang SC` and `Hiragino Sans GB` on
macOS, `Microsoft YaHei` and `SimHei` on Windows, `Noto Sans CJK` and
`WenQuanYi Zen Hei` on Linux). For a script the default list does not cover,
name a font that has it as the first entry of the stack.

The fallback list is deliberately separate from `DEFAULT_FONT_STACK`, the
theme's default family stack. The theme's stack decides which face text is
*primarily* drawn with and must stay Latin-first; the fallback list only fills
in glyphs and must never become the primary face. Keeping them apart means a
new coverage entry cannot silently change how a default chart looks.

## Text size is an em size

`font_size` means the same thing in every backend: an **em size**, exactly like
CSS `font-size` and the PDF text operator. A chart with an 18px title has an
18px em in PNG, SVG and PDF.

That is worth spelling out because the Rust text stack does not work in ems.
`ab_glyph`'s `PxScale` is the pixel height of the *line box* (`ascent - descent`),
and the ratio between that height and the em square differs per face: 1.0 for
many CJK fonts, about 1.21 for Inter, about 1.4 for PingFang SC. Feeding
`font_size` straight into `PxScale` therefore makes text 17-30% smaller than the
same font size in SVG, and the error depends on which font the text happens to
use.

`core::font::em_scale` performs the conversion, and it is applied **per face**.
The per-face part matters as much as the conversion: one scale shared by a whole
fallback chain draws a fallback glyph at a different em than the primary face's
glyphs, so in `m²` the `m` and the `²` would differ in height by 15-30%.

Layout budgets use the same em. `estimate_text_width` measures East Asian wide
characters as one em, so the rectangle a title or legend entry is allotted
matches the size it is drawn at.

## CJK and other large scripts

Charton bundles only the small Latin **Inter** face. A full CJK font is several
megabytes per weight, and the text a chart draws is runtime data, so no subset
can be chosen at build time; shipping one would dominate the binary and the WASM
payload. CJK is therefore served by the operating system's fonts through the
fallback chain, just as a browser does it.

So on a machine with a CJK font installed, Chinese, Japanese and Korean text
just works. On a machine without one — a slim container or CI image, for
instance — those characters have no face to fall back to and are drawn as
`.notdef` boxes, while the render still succeeds. Install a CJK font in the
image (for example a Noto CJK package), or name one that is present as the first
entry of the stack.

## Weight and slant

Beyond the family, text carries a `font_weight` and a `font_style` using the
CSS values: weights are `"normal"`, `"bold"` or a numeric value such as
`"600"`, and slants are `"normal"`, `"italic"` or `"oblique"`. They are
matched by `fontdb` against the faces of the chosen family, so bold or italic
text picks the family's bold/italic member when one is installed and the closest
available variant otherwise.

For `mark_text` both are set through the builder:

```rust
chart
    .mark_text()?
    .configure_text(|t| {
        t.with_text("p < 0.05")
            .with_font_family("Georgia, serif")
            .with_weight("bold")
            .with_italic(true)
    });
```

The renderers choose weight and slant per role rather than exposing a slider:
chart titles, facet headers and cartesian axis titles are bold, while tick
labels, legend entries and geospatial labels are normal, and all of them are
upright. Both eager backends honour weight and slant; SVG just forwards them as
`font-weight`/`font-style` attributes.

One caveat: the bundled Inter ships as **Regular only**, so the default theme's
bold titles keep using the regular face. A family that provides a bold or italic
member produces the matching face as soon as it is selected (for example
`with_title_family("Arial")` gives real bold titles).

## An unknown family is an error

If *none* of the named families exists — and the stack names no generic — the
renderer returns `ChartonError::Font` instead of silently substituting a
different font:

```text
Font error: no font matched the family stack "NoSuchFontXYZ123";
install one of the named fonts or use a generic family such as "sans-serif"
```

Mixing available and unavailable names is allowed (that is normal CSS): only a
stack where **nothing** resolves is rejected. Silently substituting a different
font would hide the user's intent, so an all-unknown stack is an error rather
than a fallback. The error is returned from `to_png()`/`to_pdf()`; SVG does not
raise it because the browser resolves the stack (see below).

## Backend behaviour

| Backend | Who resolves the stack                          | Missing family |
| ------- | ----------------------------------------------- | -------------- |
| PNG / raster | Charton (`tiny-skia` + `ab_glyph` + `fontdb`) | `ChartonError::Font`; per-glyph fallback |
| PDF | Charton (`krilla`)                              | `ChartonError::Font`; per-run fallback |
| SVG | the viewer (browser)                             | none — the browser's own fallback applies |
| WGPU | the host renderer                                | follows the host |

For PNG and PDF the candidate chain is walked as described above. PDF's text API
draws one font per call, so consecutive characters that share a face are emitted
as a single run and the string is split wherever the face changes. SVG writes a
`font-family="..."` attribute and leaves the choice to whoever renders the SVG;
it therefore accepts any stack, including names that are not installed locally.
The WGPU backend uses the GPU for geometry and defers text to the host — either
CPU compositing (which goes through the raster path) or the browser's canvas on
the web.

## Configuring fonts

Fonts are configured through the theme; a `mark_text` mark carries its own
family and can override the theme for that mark. The four theme fields cover the
text the engine draws itself:

```rust
let themed = chart.configure_theme(|t| {
    t.with_title_family("Georgia, serif")
        .with_label_family("Georgia, serif")
        .with_tick_label_family("Inter, sans-serif")
        .with_legend_label_family("Inter, sans-serif")
});
```

Each accepts the same stack syntax. When only one field is changed, the others
keep the default stack.

## Notes for contributors

- `src/core/font.rs` is the single source of truth. New text-drawing backends
  should resolve through `resolve_candidates` rather than querying `fontdb`
  directly, or the backends will drift apart again.
- Text is measured and drawn through `core::font`. `em_scale` converts
  `font_size` (an em size) into `ab_glyph`'s face-specific `PxScale`, `shape`
  selects the face per character and returns advances and kerning in pixels,
  `line_metrics` gives the baseline, and `run_width` totals a run. The raster and
  PDF backends must draw through these functions, or they will drift apart in
  size and spacing again. Never pass `font_size` to a font library as a line
  height, and never share one scale across a fallback chain.
- `DEFAULT_FONT_STACK` is the theme's visual default; `FALLBACK_FONT_STACK` is
  the coverage list appended to whatever the user asked for. Add coverage
  families to the second one.
- `RenderBackend::draw_text` is fallible so that a backend which resolves fonts
  itself can report a bad family. Backends that defer font selection return
  `Ok(())`.
- There is no runtime font-registration API: the available faces are the system
  fonts plus the bundled Inter.
