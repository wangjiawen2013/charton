# Changelog

All notable changes to the `charton` library are documented here. The project
follows [Semantic Versioning](https://semver.org/). Releases before 0.7.0
predate this file.

## [0.8.5] - 2026-10-17

### Added

- **`MarkText::with_font_family(...)`, `with_italic(bool)` and `with_font_style(...)`**
  for text marks, plus a `font_style` field on `TextConfig` that backends read.
  `with_font_family` completes the text mark, whose family field previously had
  no setter.
- **`ScaleDomain::DiscretePositions`** records both the category labels and the
  numeric position extent of an axis built with `with_category_labels`.
  `ScaleDomain::discrete_labels()` and `ScaleDomain::is_discrete()` make the
  two discrete variants easy to handle; downstream exhaustive matches must
  handle the new variant.

### Fixed

- **A discrete category axis now covers the numeric positions its marks use.**
  `with_category_labels` builds a discrete axis from the category list, but a
  ridgeline with overlap, a dodged lane, or any composed mark can sit outside
  the integer category indices. The scale now trains on the real position
  extent as well and expands to include it, so those charts no longer need a
  hand-written `with_expansion` to avoid being clipped by the panel boundary.
  On these category axes, manual expansion still works and now only controls
  the amount of breathing room.
- **Font weight and slant are now resolved to the matching face.** The raster
  and PDF backends previously ignored `font_weight`; they now query `fontdb`
  with the requested weight, stretch and slant, so bold text selects a bold face
  when the selected family provides one. (The bundled Inter is Regular only, so
  the default theme's bold titles keep using the regular face.) SVG already
  forwarded the values as attributes.
- **CSS-style font family stacks are resolved in the raster and PDF backends.**
  A `font_family` may be a stack such as `"Inter, 'Segoe UI', sans-serif"`.
  Both backends now try the names left to right and map the generic names
  (`sans-serif`, `serif`, `monospace`, `cursive`, `fantasy`) onto the system
  families. Previously the raster backend passed the whole stack to `fontdb` as
  a single family name, so every requested system font was ignored and text
  fell back to the bundled Inter.
- **Missing glyphs fall back per character.** A stack now resolves to an ordered
  candidate chain (the requested faces, a curated system fallback, then the
  bundled Inter). The raster backend draws each character with the first face
  that contains it and the PDF backend splits the string into runs, so Latin,
  CJK and symbols such as `²` render correctly even when the primary font does
  not cover them.
- **Raster text is drawn at the requested em size, and its outlines match the
  advances that position it.** The raster backend scaled outlines by
  `units_per_em` while positioning them with advances based on the font's line
  box, so a glyph could be wider than its own advance: a label such as `2.0`
  read as `20` because the oversized `2` covered the decimal point. Aligning the
  outline with the advance was not enough on its own, because `ab_glyph`'s
  `PxScale` is a line-box height, not an em: the same mismatch made PNG text
  17-30% smaller than SVG and PDF at the same `font_size` (Inter's line box is
  1.21x its em, Microsoft YaHei's 1.32x, PingFang SC's about 1.4x). Text is now
  shaped through one em-based layout in `core::font` — `em_scale` per face,
  `shape`, `line_metrics`, `run_width` — so one em is `font_size` pixels in
  every backend and a fallback glyph is the same size as the primary face's
  glyphs (`m` and `²` in `m²` used to differ by 17%).
- **CJK labels are measured at their real width when laying out.** The text
  width estimate treated every non-ASCII character as 0.55em, so a four
  character Chinese legend entry was allotted about 2.2em while it draws at 4em
  and was clipped at the figure edge (in SVG as well as PNG). East Asian
  Wide/Fullwidth characters now count as one em.
- **Linux and CJK-system fallbacks are actually reachable.** The fallback chain
  reused the theme's default family stack, which is Latin-first and lists no
  Linux CJK family, so a container with only `Noto Sans CJK` installed still
  drew `.notdef` boxes. Coverage now has its own list
  (`FALLBACK_FONT_STACK`) with the CJK families of every supported platform,
  kept separate from the theme's default so adding coverage cannot change how a
  default chart looks.
- **Unknown font families are no longer swallowed.** A stack in which no name
  resolves now returns `ChartonError::Font` from `to_png()` / `to_pdf()` instead
  of silently drawing with Inter. Mixed stacks (some names missing) still work,
  and generic names such as `sans-serif` resolve through the bundled face.

### Changed

- **`with_font_scale` now scales the text layout, not just the glyphs.** Axis
  title/tick padding, the main title padding, legend spacing, and facet strip
  padding scale with the font. Enlarging text without its surrounding gaps made
  axis titles crowd their tick labels; the Nushell plugin's default `1.5` was
  visibly tighter than the library at `1.0`. The default layout is unchanged
  because the scale defaults to `1.0`.
- **`Theme::label_padding` is now the whole tick-label ↔ axis-title gap.** The
  renderer previously added a hidden 5px constant to it; that constant is gone
  and the default `label_padding` is `10px`, so the default gap is unchanged.
  A custom `with_label_padding` value now describes the whole gap rather than
  half of it.
- **Reversing a numeric category axis is now rejected.** `with_reverse` on an
  axis built with `with_category_labels` would flip the labels without flipping
  the numeric positions, so every mark would sit under the wrong label. The
  engine now reports the unsupported combination instead of drawing a wrong
  axis; reorder the categories instead.
- **Raster text is slightly larger, and uses no hidden letter spacing.** With
  the em fix above, PNG text grows by the font's line-box ratio (about 17% for
  Inter, 24% for Microsoft YaHei). The raster backend also added a hard-coded
  0.3px of tracking after every character that SVG and PDF never applied, and
  that pushed centre- and right-anchored text past its own measured width; it is
  gone. PNG images will look different. SVG and PDF output is unchanged apart
  from the CJK label-width fix above.
- **Raster output is now WYSIWYG: `with_size(w, h)` produces a `w × h`
  image.** `scale_factor` now defaults to `1.0` instead of `2.0`, so the size
  you configure is the size you get. Sharpness is no longer the caller's
  problem: `to_png()` and the wgpu PNG export render internally at 2x and
  box-downsample, skipping the extra pass for very large images or when the
  requested density is already >= 2x.
- **`with_scale_factor` is now purely a pixel-density knob** for explicitly
  requesting a higher-density asset (Retina, print, terminal image fitting). It
  no longer affects the default output size or anti-aliasing. Its purpose and
  interaction with `with_size` are now documented on the method.
- **Raster size arithmetic no longer truncates fractional scale factors**
  with `as u32`; internal and output sizes are rounded consistently.
- **`RenderBackend::draw_text` now returns `Result<(), ChartonError>`** so that
  backends which resolve fonts themselves (PNG, PDF) can report a bad family.
  SVG and WGPU, which defer font selection to the viewer or host, return
  `Ok(())`.

### Documentation

- **New "Fonts & Text Rendering" chapter** in the mdbook documents the stack
  syntax, the fallback chain, the strict unknown-family error, the per-backend
  behaviour, and why CJK is served from system fonts rather than bundled.

## [0.8.4] - 2026-10-11

### Added

- **`Theme::with_font_scale(factor)`** multiplies every font size (title, axis
  labels, tick labels, legend, facet labels) in one call. Handy for rendering
  the same chart at a larger text ratio, e.g. when fitting it into a terminal.
- **`MarkRect::with_cell_padding(padding)`** grows (or shrinks) each heatmap
  tile on every side, in device pixels. The default is `0.5`.
- **`RowAccessor::entries()`** returns the current row as `(column name, value)`
  pairs, so a `transform_calculate` callback can serialise the whole row (for
  example to hand it to an external expression evaluator).

### Fixed

- **Heatmap tiles no longer show white seams.** The raster backend drew a
  hairline for a zero-width stroke; it now skips zero-width strokes, matching
  the SVG backend. `rect` / `density_2d` also overlap their tiles by the default
  `cell_padding` so anti-aliasing cannot leave a gap between neighbours.

## [0.8.3] - 2026-10-06

Lockstep version bump so the library and its Nushell plugin keep one version
number. The library itself is unchanged from 0.8.2; the fixes in this release
are in `nu_plugin_charton` (see its changelog).

## [0.8.2] - 2026-10-05

Packaging and discoverability release. There are no API changes; charts render
identically to 0.8.1.

### Changed

- **Crate metadata.** `Cargo.toml` now declares `keywords` (`plotting`,
  `charts`, `visualization`, `polars`, `grammar-of-graphics`), `categories`
  (`visualization`, `science`, `graphics`, `wasm`), `homepage`, `documentation`
  and `readme`. The `description` now names the distinguishing features
  (Altair-style declarative plotting, Polars, SVG/PNG/PDF, Wasm) instead of
  internal architecture. This affects placement and ranking on crates.io and
  lib.rs only.

### Documentation

- **docs.rs builds with optional features enabled.** A new
  `[package.metadata.docs.rs]` section selects `png`, `pdf`, `geo`, `arrow` and
  `parallel`, so the APIs behind those features are visible on docs.rs — they
  were previously hidden because docs.rs built with no features. `wgpu` is
  intentionally excluded to keep the docs.rs build within its time budget, and
  will be added once a local `cargo doc --all-features` proves it fits.

## [0.8.1] - 2026-10-05

### Added

- **Dashed paths and polygons.** `mark_path` / `mark_polygon` (`MarkGeoPath`)
  gained `with_dash([dash, gap])`. The pattern follows the SVG
  `stroke-dasharray` rules and is measured along the path's arc length. The
  geometry renderer previously hard-coded an empty dash, so a dashed contour
  (or a dashed polygon outline) was impossible on any backend. It now works on
  SVG, raster, PDF and WGPU.
- **Convenience composite marks: `mark_violin`, `mark_density`, `mark_contour`
  and `mark_density_2d`.** Each is a *thin recipe* over the existing public
  transforms and renderers — there is no new statistic and no new drawing path:

  | Mark | Expands into |
  |---|---|
  | `mark_violin` | `transform_density` + `transform_band` → polygon renderer |
  | `mark_density` | `transform_density` + area prep → area renderer |
  | `mark_contour` | `transform_contour` → open-path renderer |
  | `mark_density_2d` | `transform_density_2d` + rect prep → rect renderer |

  Inputs come from the encodings, so e.g. `mark_violin` reads the value from
  `y`, the category from `x` and the lanes from `color`. A faceted chart re-runs
  the statistic per panel. Each mark is covered by a test proving it renders
  byte-for-byte identically to the hand-written composition, and can be tuned
  with `configure_violin` / `configure_density` / `configure_contour` /
  `configure_density_2d`.
- **Per-channel display labels.** The `X`, `Y`, `Color`, `Shape` and `Size`
  encodings gained `with_label(…)`. Axis and legend titles now resolve in the
  order: explicit chart label (`with_x_label` / `with_color_label` / …), then the
  channel label, then the field name. The composite marks use this to keep the
  original column names on the axes and legends after they rewrite the encoding
  to generated columns (`x`, `y`, `density`, `level`, …).
- **Floating bars, cumulative sums and reversed axes.** Three general
  capabilities, one per grammar layer, used by the new comparison and time
  recipes:
  - **geometry** — `mark_bar` accepts a `y2` bound, so a bar can *float* between
    two values (a waterfall step, a candlestick body) instead of growing from
    zero. With `y2` present, `color` is an attribute rather than a dodging lane
    (no Cartesian product, no phantom bars), and the y axis is **not** forced to
    include zero — the interval keeps the axis to itself.
  - **statistic** — `transform_window` gains `WindowOnlyOp::CumulativeSum`, a
    running total accumulated in row order (a waterfall's total).
  - **scale** — `alt::y("rank").with_reverse(true)` flips an axis (`alt::x(…)
    .with_reverse(true)` for x). A reversed domain still expands and ticks
    correctly, so a rank axis can put rank 1 at the top. Passing a continuous
    domain backwards (`with_y_domain(max, min)`) does the same.
- **`mark_point` dodges only on a categorical axis.** Colour-grouped points sit
  side by side only when the x axis is discrete: dodging is a category concept,
  and on a continuous axis the offset was scaled by an arbitrary data unit (so a
  narrow numeric range distorted the chart). On a continuous axis the points now
  stay on their own x, lining up with a rule, line or area through them.
  `configure_point(…).with_dodge(false)` still turns the lane off on a
  categorical axis — a dumbbell uses it to keep both ends on its segment.
- **One named default gap for the outline/point family.**
  `Position::DEFAULT_DODGE_SPACING` (`0.2`) is now the single default used by
  `mark_boxplot`, `mark_point` and the band/quantile boxes; fill/interval marks
  (`mark_bar`, `mark_errorbar`) keep `0.0` so they tile. The dodge arithmetic was
  already shared via `Position::offset`; `tests/test_lane_alignment.rs` now also
  locks the layered bar↔error-bar pair.
- **One-sided and overlapping bands: `BandTransform::with_side` and
  `with_overlap`.** `with_side(BandSide::{Both, Left, Right})` chooses which bank
  of the river a band occupies (`Both` is the symmetric violin; `Right` / `Left`
  keep one side, whose flat edge is the centre line). `with_overlap(f)`
  multiplies the width so a band can grow past its lane and overlap its
  neighbour. Both are general geometry/scale options on `transform_band`, and are
  reached from `mark_violin` too via `configure_violin(…).with_side(…)` /
  `.with_overlap(…)`. `with_split` is now explicitly a *two-group* option: a lone
  group stays symmetric instead of silently becoming one-sided. A **ridgeline**
  is `transform_density` + `transform_band(…).with_side(BandSide::Right)` +
  `with_overlap(f)` + `coord_flip`; it deliberately stays a recipe rather than a
  mark — see
  [Composing Your Own](docs/src/gallery/primitives.md#a-ridgeline-from-a-half-band)
  and
  [the mark firewall](docs/src/concepts/design_rules.md#the-firewall-a-mark-must-change-a-cell).

### Fixed

- **WGPU closed-polygon outlines no longer drop their closing edge.** A `Complex`
  path is a closed region, which the SVG/raster/PDF backends close with `Z`; the
  WGPU path shader extrups one quad per consecutive pair and never wrapped, so a
  violin, box or custom polygon outline was left open on the GPU backend.
- **WGPU path strokes now honour the dash pattern.** Dashed contours and area
  outlines render dashed on the GPU backend instead of solid.

### Changed

- **Missing values follow the ggplot2 / Vega-Lite rule, consistently across
  every mark.** What happens to a null (or `NaN`) now depends only on the
  channel it appears in:
  - `x` / `y` — the row is dropped, since there is no position to draw it at.
    On `mark_line` / `mark_area` this breaks the path, so an outage in a time
    series shows as a visible gap.
  - `color` — the row is kept and drawn as a reserved grey `NA` level, listed
    last in the legend. Works for discrete and continuous colour scales.
  - a transform's non-positional grouping (a violin's `group`, a window's
    `groupby`) — kept as an `NA` group.
  - `shape` / `size` — the row is kept and falls back to the default.

  `ColumnVector::unique_values` remains the positional category list (no missing
  values). The new `ColumnVector::labels_with_missing`, `label_with_missing` and
  `has_null` provide the non-positional list and its per-row labels. See
  [Missing Values & Gaps](docs/src/concepts/missing_values.md).

## [0.8.0] - 2026-10-03

### Breaking changes

- `FacetMetrics` gained `per_column_left` and `per_row_bottom` (both `Vec<f64>`)
  so free facets can size axis tracks per column/row. It still derives `Default`,
  so `FacetMetrics { axis_left, axis_bottom, ..Default::default() }` keeps
  working.
- The `Layer` trait gained a required `without_source_data` method. `Chart<T>`
  implements it; external `Layer` implementations must provide it.

### Added

- **Contour plots from general parts.** Two orthogonal primitives, not a
  contour-specific mark or transform:
  - `mark_path` — the general *open* polyline geometry. It connects the rows
    that share a `path_group` in row order (no sorting, no closing) and strokes
    them with the colour channel. `mark_polygon`/`mark_geoshape` are the closed,
    filled form of the same geometry.
  - `transform_contour` — marches squares over a regular `x`/`y`/`z` grid and
    emits `(x, y, path_group, level)` iso-lines.
  - `transform_density_2d` — estimates a bivariate kernel density onto a grid,
    so scattered points become a density field. It feeds the same contour
    transform: `scatter → density_2d → contour → mark_path` is the classic
    density contour (`kdeplot` / `geom_density_2d`).

  A contour plot is `transform_contour` + `mark_path`; the same pieces draw
  custom curves and parallel coordinates.
- **Violin plots, composed from basic parts.** There is deliberately no violin
  mark and no violin-specific transform. Two recipes cover every layout:

  - *Reuse* (Vega-Lite / Altair style): `transform_density` + `mark_area` with
    `StackMode::Mirror` (`with_stack("mirror")`) draws single and faceted
    violins with no violin-specific code.
  - *Geometry* (ggplot2 `stat_ydensity` style): `DensityTransform::with_groupbys`
    groups by `[category, group]` at once; `transform_band` is the general band
    geometry that draws the symmetric outline and applies a `Position`;
    `transform_quantile_box` adds the inner inter-quartile box and median as a
    second polygon layer. All reuse the shared KDE core.

  `mark_polygon` (an alias of the shared polygon renderer) is the geometry.
- **General band and box geometry.** `transform_band` turns a value axis and a
  half-width column into a symmetric `centre ± width` polygon, with a `Position`
  for dodging and a `split` mode for two-sided bands. `transform_quantile_box`
  computes per-group quartiles and emits the inter-quartile box and median as
  polygons. Neither is violin-specific: they are the reusable geometry behind
  violins, rainclouds and any placed ribbon.
- `Position`, a data-space position adjustment (`Identity` / `Dodge`). Offsets
  are measured in category steps, so every geometry can share one layout.
- `BandTransform::with_split` draws two groups as the left and right halves of
  one band.
- `DensityTransform::with_groupbys` groups a density estimate by several fields
  at once (for example `["Sex", "Species"]`), which is what dodged and split
  violins need.
- `StackMode::Mirror` / `with_stack("mirror")`: draws each series symmetrically
  around zero (`-value/2 .. +value/2`), independently of the others. This is the
  Vega-Lite `stack: "center"` behaviour applied per series.
- `alt::x(...).with_category_labels("field")`: a discrete position scale that
  reads numeric positions while drawing integer ticks labelled from `field`.
- `ExplicitTick::Labeled` and `IntoExplicitTicks` for `Vec<(f64, &str)>` /
  `Vec<(f64, String)>`, for hand-labelled numeric ticks.
- `Dataset::column_arc`: a shared (`Arc`) handle to a column.
- **`DensityTransform::with_trim`.** Optional per-group trimming: with
  `with_trim(true)` each group is evaluated over its own observed `[min, max]`
  (matching ggplot2 `geom_violin`, `trim = TRUE`, and Altair's violin), so
  violins of different spread get different heights and no near-zero tails.
  The default stays `false`, so a density plot keeps its shared, 30 % extended
  tails (`geom_density`). The violin examples opt in.

### Changed

- **Violin width now matches the box plot and point marks.** A violin has a
  separate *span* (total width of one category's group, default `0.7`) and
  *width* (maximum width of a single violin, default `0.5`), dodged violins use
  the same 20 % gap (`Position::dodge()`), and the violin takes the same
  discrete-axis padding as the box plot. A grouped violin group is now exactly
  as wide as a grouped box plot or scatter, and its leftmost violin no longer
  touches the axis.
- **Dodge has one implementation.** `Position::item_width` / `Position::offset`
  own the side-by-side arithmetic; the bar, box plot, error bar and point
  renderers call them instead of each re-deriving the same formula. Output is
  byte-for-byte unchanged.
- **Statistics run per facet panel.** A layer keeps its pre-statistic rows and a
  panel re-runs the statistic on its own subset, so stacking, normalisation and
  density never leak across panels.
- **Facets honour `Free` / `FreeX` / `FreeY`.** Each panel trains its own x
  and/or y scale from that panel's layers; aesthetic scales stay shared. Axis
  tracks are sized per panel column and row, so a panel with short labels does
  not reserve space only another panel needs. `FacetMetrics` now carries
  per-column/per-row extents.
- **Zero-copy columns.** The bar, area and density transforms capture a
  lightweight *type prototype* instead of copying a whole column to restore its
  output type; `take_rows` shares columns for a full in-order slice. A
  non-faceted chart releases each layer's pre-statistic snapshot before drawing,
  so a long-lived chart does not keep the raw rows a per-panel statistic would
  need.

### Fixed

- **A transform never silently overwrites its own output.** When a generated
  column name would clash with a copied-back category/group column (for example
  a category column already called `x`), the transform now returns a clear error
  instead of overwriting the computed column.
- **A mirrored or centered area no longer sits flush against the axis.** A
  violin built as `mark_area` with `stack: "mirror"` / `"center"` is symmetric
  about zero, but it was given the asymmetric "grows from a baseline" padding,
  so its lower half touched the axis (a lone violin had no gap at all, unlike a
  box plot). It now gets the same 5 % padding on both ends.
- The polygon renderer no longer applies the shared colour scale to layers that
  did not map a colour (a violin's inner box used to inherit the outline's
  colours).

### Documentation

- New *The Layer Pipeline: Data → Stat → Position → Geom* concept chapter, and
  the *Statistical Distributions* and *Box & Violin Combinations* gallery pages
  rewritten around it. See `examples/violin.rs`, `grouped_violin.rs`,
  `split_violin.rs` and `raincloud.rs`.
- **New Cookbook section.** A [recipe index](docs/src/gallery/index.md) plus one
  page per chart family — Violin, Distributions, Bars & Error Bars, Line, Area,
  Strip/Rug, Circular, Heatmaps, Scatter, Contours, Faceting, Geospatial. Every
  code block is a real, compiled `examples/*.rs` pulled in with `{{#include}}`,
  so the book cannot drift from the API.
- **Curated gallery images.** Smoke-test examples moved into `tests/`, and tests
  and dev demos now write to `target/` instead of the repository. CI now fails
  if a committed image under `docs/src/images/` is not referenced by a Markdown
  page.
- **New *Transforms & Columns* reference.** Every transform's input and output
  columns are now documented in one place
  (`docs/src/grammar/transforms.md`), so a composed pipeline never relies on a
  hidden column name (`density`, `path_group`, `level`).

[0.8.0]: https://github.com/wangjiawen2013/charton/releases/tag/v0.8.0

## [0.7.1]

No API changes. This release bumps the version to stay in lockstep with
`nu_plugin_charton` 0.7.1.

[0.7.1]: https://github.com/wangjiawen2013/charton/releases/tag/v0.7.1

## [0.7.0]

### Added

- `ThemeMode` (`Light` / `Dark`), with `Theme::light()`, `Theme::dark()`,
  `Theme::with_mode()`, and `Theme::is_dark()`. A mode only changes colors, so
  switching between light and dark leaves layout and typography untouched.

### Changed

- Tightened the default canvas margins — `top` 0.05 → 0.025, `right` 0.03 →
  0.02, `bottom` 0.08 → 0.03, `left` 0.06 → 0.025 — so charts use their space
  better. The plot area is now larger; set margins explicitly to restore the
  previous look.
- Reduced the per-axis edge buffer from 10px to 4px. The canvas margins already
  provide the outer breathing room, so the buffer is now only a small safety
  gap.

### Fixed

- Legend symbols for size-only and shape-only legends now use the theme's
  legend ink instead of a hard-coded `#333333`, which was nearly invisible on
  dark backgrounds.

[0.7.0]: https://github.com/wangjiawen2013/charton/releases/tag/v0.7.0
[0.8.3]: https://github.com/wangjiawen2013/charton/releases/tag/v0.8.3
[0.8.2]: https://github.com/wangjiawen2013/charton/releases/tag/v0.8.2
[0.8.1]: https://github.com/wangjiawen2013/charton/releases/tag/v0.8.1
