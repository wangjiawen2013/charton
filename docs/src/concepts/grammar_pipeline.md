# The Layer Pipeline: Data → Stat → Position → Geom

Charton describes a chart as a stack of **layers**. Each layer answers four
independent questions, in this order:

| Question | Stage | Lives in |
|---|---|---|
| What values are there? | **stat** | `transform` |
| Where does each mark go? | **position** | `position` |
| What shape is drawn? | **geometry** | `mark` + `render` |
| How does a value become a pixel? | **scale** + **coordinate** | `scale`, `coordinate` |

Keeping the four stages apart is the whole point of a grammar of graphics: a
handful of basic parts can then describe almost any chart. This chapter explains
each stage and shows how a violin plot is nothing more than the composition
`density estimate + dodge + polygon`.

## 1. Stat — summarise the data

A statistic turns raw rows into the numbers a geometry needs. Examples:

* a histogram counts rows per bin;
* a box plot reduces each group to five numbers (min, q1, median, q3, max);
* a **violin** estimates the density of a numeric column.

Stats are ordinary transforms that run on the `Dataset` **per group**. A group is
the combination of the discrete aesthetics (colour, facet, and so on). In code:

```rust
chart!(iris)?
    .transform_violin(ViolinTransform::new("sepal_length"))? // stat
    .mark_polygon()?                                         // geom
    .encode((alt::x("x"), alt::y("y"), alt::path_group("violin_id")))?
```

`transform_violin` writes three columns — `x`, `y` and a `path_group` id per
violin. It does **not** know or care that a polygon will be drawn; it only
produces a table.

## 2. Position — decide where marks sit

When several marks share one category, they must be arranged. In a violin plot
that means several distributions inside the same x slot:

* **identity** — they overlap. Use this when the split is handled by facets;
* **dodge** — they sit side by side.

`Position` works in **data space**, measured in *category steps* (`1.0` is one
full slot). It never touches pixels, so every geometry — bars, boxes, points,
error bars, violins — can reuse the same layout:

```rust
ViolinTransform::new("Body Mass (g)")
    .with_category("Sex")     // x position
    .with_group("Species")    // one curve per species
    .with_position(Position::dodge())
```

Because the shift is a data-space number, a violin's centre is simply
`category_index + dodge_offset`. No renderer has to re-implement dodge.

The same helpers back the existing marks. The bar, box plot, error bar and point
renderers resolve their lanes through `Position::item_width` and
`Position::offset`; they only convert the resulting category-step shift into
pixels. Adding a new dodge rule (or a new mark) is therefore a change in one
place.

## 3. Geometry — draw the shape

The geometry consumes the final coordinates and emits backend primitives
(circles, rectangles, paths). The polygon geometry is shared by many pictures:
maps, custom shapes, and violins all use the same `mark_polygon` renderer. A
violin is just a closed outline:

```
right side: (centre + width, y)  going up
left  side: (centre - width, y)  coming back down
```

Because the width and the centre were already solved by the stat and the
position, the polygon renderer stays trivial.

## 4. Scale and coordinate — the final mapping

Scales turn data values into the normalized `[0, 1]` range; coordinates turn
that into pixels (and handle flips and polar projections). These stages are
unchanged by the stat/position split, which is exactly the benefit: new charts
are added by combining existing stages, not by teaching every backend a new
trick.

### Numeric positions with categorical labels

A dodged violin places its categories at integer positions (`0, 1, 2, …`) and
then adds fractional offsets. The encoded column therefore holds *numbers*, but
the axis should show *category names*. Tell the encoding where the categories
live:

```rust
alt::x("x").with_category_labels("Sex")
```

This makes the axis a **discrete position scale**: it draws one integer tick
per category, labelled from the given column, while the numeric positions
(including the fractional dodge offsets) are read through the same scale. This
is the same idea as ggplot2's discrete position scale — categorical labels
backed by numeric positions.

Labelled ticks remain available when you prefer to set the labels by hand:

```rust
.with_x_ticks(vec![(0.0, "Female"), (1.0, "Male")])
```

## Mapping the pipeline to code

| Stage | Public API | Implementation |
|---|---|---|
| stat | `transform_density`, `transform_violin`, a box plot's built-in summary | `src/transform/*`, `src/stats/*` |
| position | `Position::{Identity, Dodge}`, `with_stack(...)` | `src/position.rs`, `src/encode/y.rs` |
| geometry | `mark_polygon`, `mark_area`, `mark_point`, ... | `src/mark/*`, `src/render/*_renderer.rs` |
| scale | `Scale`, `with_scale`, `with_category_labels` | `src/scale/*` |
| coordinate | `coord_flip`, `with_coord`, `CoordSystem` | `src/coordinate/*` |
| layer | `chart!`, `.mark_*()`, `.encode(...)`, `.and(...)`, `.facet(...)` | `src/chart.rs`, `src/core/composite.rs` |

A `Mark` is a *configuration* type (for example `MarkArea`, `MarkBoxplot`); its
`MarkRenderer` turns the already-placed coordinates into backend primitives. The
backend only ever sees circles, rectangles, lines and paths — never a "violin"
or a "box plot".

## Per-panel statistics

A statistic is computed on the rows that appear in the panel it belongs to, not
on the whole dataset. This is what makes faceting safe for cumulative or
panel-relative summaries.

Concretely, each layer keeps a copy of the data as it was *before* its statistic
ran. When the engine materialises a panel it takes that panel's rows from the
copy and re-runs the statistic on them. A cumulative statistic (stacking) or a
panel-relative one therefore never sees data from another panel.

The copy is cheap: columns are `Arc`-backed and shared, so taking the snapshot is
a reference-count bump rather than a duplication of the data. A chart with no
facets never re-runs a statistic, so it releases the snapshot before drawing;
only a faceted chart keeps it.

The payoff is visible with the pure Vega-Lite violin recipe:

```rust
.transform_density(DensityTransform::new("sepal_length").with_groupby("species"))?
.mark_area()?
.encode((alt::x("sepal_length"),
         alt::y("density").with_stack("center"),
         alt::color("species")))?
.facet(FacetSpec::wrap("species"))
```

Each panel runs its own `center` stack, so every violin is symmetric. Before
per-panel statistics the stack was computed once for all species and then
filtered, which shifted each panel's violin.

Scales are still trained globally (shared across panels), which is the usual
*fixed scales* behaviour. For a cumulative statistic that can leave a little
extra room on the axis, but it never clips a panel.

### Fixed and free scales

Every panel shares one scale per channel by default (*fixed*). `FacetSpec` can
ask for independent scales with `with_strategy("free")`, `"free_x"` or
`"free_y"`. A free positional scale is trained from that panel's own layers,
after the panel's statistic, so a panel whose values sit high on the axis gets
its own range instead of being flattened by the others. Aesthetic scales
(colour, shape, size) stay shared so the legend remains unified.

Free scales also size their axis tracks per panel: each column reserves as much
room on the left as its widest y axis needs, and each row as much room below as
its tallest x axis needs. A panel with short labels therefore no longer reserves
space that only another panel's labels require.

## Worked example: violin plots

A violin can be built two ways, both faithful to how the wider ecosystem does
it. Prefer reuse; add a dedicated stat only when the grammar cannot express the
layout.

| Layout | Stat | Position | Geom |
|---|---|---|---|
| single | `transform_density` (reused) | identity + `stack: "mirror"` | `mark_area` |
| faceted | `transform_density(group)` (reused) + `facet` | identity + `stack: "mirror"` | `mark_area` |
| dodged | `transform_violin(value, category, group)` | `Position::Dodge` | `mark_polygon` |
| split | `transform_violin(..., split: true)` | identity | `mark_polygon` |
| raincloud | `transform_violin` + `transform_violin_box` | identity | `mark_polygon` + `mark_point` |

The first two rows are the Vega-Lite / Altair recipe: the ordinary density
transform plus an area mark whose values are mirrored around the centre. The
`"mirror"` stack mode is what makes the area symmetric per series.

The third row is the ggplot2 recipe (`stat_ydensity`). It is needed because a
dodged violin puts the category on x and groups by two fields at once, which the
density transform cannot express. The new stat still reuses the shared KDE core,
so no statistics are duplicated.

### When do you need a new stat?

Add a stat only when the existing transforms cannot produce the required
columns. Density, binning, quantiles and regression are reusable; a stat that
only rearranges their output (mirroring, placing, dodging) belongs to the
position and geometry stages instead.

Raincloud, split violins and beeswarms are further combinations of the same
parts (a density stat, a position, and one or more geometries layered with
`.and(…)`). No new "chart type" is required.

## Design notes: what this refactor changed

Charton used to hide each chart type inside its own layer: a bar layer computed
its own dodge, a box plot layer its own, and so on. Building the violin made the
stages explicit. The changes that matter to a reader of the code:

- **Dodge has one implementation.** `Position::item_width` / `Position::offset`
  own the side-by-side arithmetic, and the bar, box plot, error bar and point
  renderers call them instead of each re-deriving the same formula.
- **Statistics run per panel.** A layer keeps its pre-statistic rows (shared, not
  copied) and a faceted chart re-runs the statistic on each panel's subset, so
  stacking, normalisation and density never leak across panels.
- **Discrete position scales carry numbers.** `with_category_labels` lets a
  numeric position column display a categorical axis, so a dodged violin can be
  placed numerically while the axis still shows names.
- **Facets can share or free their scales.** `with_strategy("free")` (or
  `"free_x"` / `"free_y"`) trains a positional scale inside each panel.
- **Violin is composed, never special-cased.** There is deliberately no
  `MarkViolin`. A violin is a density stat, a position and the shared polygon;
  rainclouds and split violins are more layers on the same parts.

New chart types should follow the same recipe: add a stat if the data needs
summarising, express grouping as a position, and reuse an existing geometry.
