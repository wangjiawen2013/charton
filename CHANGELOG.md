# Changelog

All notable changes to the `charton` library are documented here. The project
follows [Semantic Versioning](https://semver.org/). Releases before 0.7.0
predate this file.

## [Unreleased]

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
  and dev demos now write to `target/` instead of the repository. The new
  `scripts/check-doc-images.sh` (wired into CI) fails if a committed image is not
  referenced by a Markdown page.

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
