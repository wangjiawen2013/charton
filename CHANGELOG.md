# Changelog

All notable changes to the `charton` library are documented here. The project
follows [Semantic Versioning](https://semver.org/). Releases before 0.7.0
predate this file.

## [Unreleased]

### Breaking changes

- `FacetMetrics` gained `per_column_left` and `per_row_bottom` (both `Vec<f64>`)
  so free facets can size axis tracks per column/row. It still derives `Default`,
  so `FacetMetrics { axis_left, axis_bottom, ..Default::default() }` keeps
  working.
- The `Layer` trait gained a required `without_source_data` method. `Chart<T>`
  implements it; external `Layer` implementations must provide it.

### Added

- **Violin plots, composed from basic parts.** There is deliberately no violin
  mark. Two recipes cover every layout:

  - *Reuse* (Vega-Lite / Altair style): `transform_density` + `mark_area` with
    `StackMode::Mirror` (`with_stack("mirror")`) draws single and faceted
    violins with no violin-specific code.
  - *Dedicated stat* (ggplot2 `stat_ydensity` style): `transform_violin` handles
    dodged grouped and split violins, which need the category on x and a
    two-field grouping; `transform_violin_box` adds the inner inter-quartile box
    and median as a second polygon layer. Both reuse the shared KDE core.

  `mark_polygon` (an alias of the shared polygon renderer) is the geometry.
- `Position`, a data-space position adjustment (`Identity` / `Dodge`). Offsets
  are measured in category steps, so every geometry can share one layout.
- `ViolinTransform::with_split` draws two groups as the left and right halves of
  one violin.
- `StackMode::Mirror` / `with_stack("mirror")`: draws each series symmetrically
  around zero (`-value/2 .. +value/2`), independently of the others. This is the
  Vega-Lite `stack: "center"` behaviour applied per series.
- `alt::x(...).with_category_labels("field")`: a discrete position scale that
  reads numeric positions while drawing integer ticks labelled from `field`.
- `ExplicitTick::Labeled` and `IntoExplicitTicks` for `Vec<(f64, &str)>` /
  `Vec<(f64, String)>`, for hand-labelled numeric ticks.
- `Dataset::column_arc`: a shared (`Arc`) handle to a column.

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

- The polygon renderer no longer applies the shared colour scale to layers that
  did not map a colour (a violin's inner box used to inherit the outline's
  colours).

### Documentation

- New *The Layer Pipeline: Data → Stat → Position → Geom* concept chapter, and
  the *Statistical Distributions* and *Box & Violin Combinations* gallery pages
  rewritten around it. See `examples/violin.rs`, `grouped_violin.rs`,
  `split_violin.rs` and `raincloud.rs`.

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
