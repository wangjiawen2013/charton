# Marks & Geometries

While Encodings and Scales define the mathematical relationship between data and space, the Mark is the physical manifestation of that relationship. A Mark is the geometric primitive used to represent a data point or a set of data points.

For the rules that classify marks into tiers and define *statistical atoms* and *composite marks*, see [Design Rules: Marks, Tiers & Statistical Atoms](../concepts/design_rules.md).

## The Role of a Mark

In Charton, a Mark is not just a drawing instruction; it is a Template that knows how to interpret resolved aesthetic values (pixels, hex codes, shapes) into final geometry.

As defined in the `Mark` trait, every mark type in Charton:

1. Identifies itself: Each mark has a unique string identifier (e.g., `"point"`, `"bar"`).
2. Provides Defaults: Marks define fallback values for properties like `stroke`, `opacity`, and `shape` if they are not explicitly mapped to data.
3. Determines Rendering Logic: Different marks require different drawing strategies—a `Point` is a single coordinate, while an `Area` is a complex polygon.

## Common Mark Types
Charton provides a rich set of marks to cover various visualization needs:

### Point Mark (`mark_point`)

The simplest mark, representing each data row as an individual geometric shape.

- Dimensions: Primarily uses `X` and `Y`.
- Aesthetics: Heavily utilizes `Shape`, `Size`, and `Color`.
- Use Case: Scatter plots and bubble charts.
- Layouts: When one axis is categorical, `.with_layout(...)` decides how the
  points share the space of their category. `"jitter"` scatters them at
  random, `"beeswarm"` packs them tightly so no two points overlap (best for
  small groups), and `"quasirandom"` spreads them with a density aware
  sequence so the cloud traces a violin outline (best for large groups).
  The quasirandom pairing can be changed with
  `.with_quasirandom_method("pseudorandom")`.

### Line Mark (`mark_line`)

Connects rows into one curve **per colour group**, after sorting each group by
`x`. This is the *series* geometry: the row order does not matter, the x value
does.

- Sorting: each contiguous segment is sorted by `x`.
- Grouping: one line per `color` value; no `path_group` needed.
- Gaps: a missing `x` or `y` breaks the line, so an outage reads as a gap
  rather than a bridge. See [Missing Values & Gaps](../concepts/missing_values.md).
- Extras: optional LOESS smoothing and step interpolation.
- Visuals: `stroke_width`, `color`, `dash`, `opacity`.
- Use it for: time series, trends, slopes, anything where the x-axis is the
  independent variable.
- Do **not** use it for a trajectory or contour: those need row order and
  `path_group`, which is [`mark_path`](#path-mark-mark_path).

### Path Mark (`mark_path`)

Connects rows **in row order** without sorting and without closing the shape,
and groups them by `path_group`. This is the "follow the data" geometry:
trajectories, contour lines, network edges, parallel coordinates.

- Order: **row order**, never sorted; the data order *is* the geometry.
- Grouping: `path_group`; colour is optional and only affects the stroke.
- Closure: open. It stops at the last vertex and strokes.
- Subpaths: `path_group(...).with_subpath(...)` draws several independent
  polylines inside one group (for example a `MultiLineString`).
- Visuals: `stroke`, `stroke_width`, `opacity`, `dash`; there is no fill.
- Configure it with `configure_path`.
- Use it for: contour lines (even a closed contour is still a line), routes,
  network edges, hand-built polylines.
- Do **not** use it when you need a filled region; that is
  [`mark_polygon`](#polygon-mark-mark_polygon).

`.with_dash([dash, gap])` draws the line as a dashed path (an empty pattern,
the default, is solid). Dashes are measured along the path, so a dashed contour
reads as a broken line on every backend, CPU and GPU alike.

### Polygon Mark (`mark_polygon`)

The closed, filled form of the same geometry: rows sharing a `path_group` are
connected in order, each ring is closed and the interior is painted with the
even-odd rule. This is the *region* geometry: violin outlines, map regions,
filled bands and hand-built filled ribbons.

- Order: **row order**, never sorted.
- Grouping: `path_group`; an optional `with_subpath(...)` splits one shape into
  an outer ring plus its holes.
- Closure: yes; every subpath is closed independently.
- Fill: even-odd. An inner ring therefore punches a real hole without relying
  on its winding direction.
- Visuals: `fill`, `stroke`, `stroke_width`, `opacity`, `dash`.
- Configure it with `configure_polygon`.
- Use it for: violin/ridge outlines, maps, filled confidence bands, arbitrary
  closed regions.
- `fill = none` does **not** turn it into `mark_path`: the ring is still closed,
  so the closing edge is still drawn. Use `mark_path` when you want an open
  line.

`.with_dash([dash, gap])` dashes the outline (the fill stays solid).

A `path_group` may contain several rings. Add
`alt::path_group("_path_id").with_subpath("_ring_id")` to give each ring its own
subpath. With even-odd filling, the outer ring paints and the inner ring punches
a hole — no winding-order convention is required. GeoJSON output already
provides `_path_id` and `_ring_id`.

### Bar Mark (`mark_bar`)

Represents data as rectangles with a **width** (the category dimension) and a
**span** (the value dimension).

- Baseline bars read `X` and `Y`: the rectangle grows from zero to `Y`.
- Floating bars add `Y2`: the rectangle spans `Y` to `Y2` instead. This is the
  geometry behind a candlestick body and a waterfall step. With `Y2` present,
  `color` is an attribute rather than a lane (no dodging layout), and the value
  axis is **not** forced to include zero — the interval keeps the axis to
  itself.
- Width and lanes come from the shared layout (`CoordLayout` + [`Position`]),
  so grouped bars sit side by side without manual offsets. Fill/interval marks
  tile (no gap); outline/point marks keep a small gap. See
  [The position rules](../concepts/design_rules.md#5-the-position-rules-车道规则).

### Area Mark (`mark_area`)

Similar to a line but filled between a baseline (Y2) and the data value (Y).

- Topology: Highlighting the volume between two series or between a series and the zero-axis.
- Gaps: A missing `x` or `y` opens the area at that point instead of bridging it. See [Missing Values & Gaps](../concepts/missing_values.md).

### Tick Mark (`mark_tick`)

Represents each row as an independent short segment, with its length and
thickness measured in **pixels**, not data units. Ticks are commonly drawn
perpendicular to the x-axis; swap `x` and `y` for horizontal ticks.

- Per row: one fixed-size glyph; rows are never connected.
- Visuals: `thickness` (stroke width) and `band_size` (segment length), plus
  `color` and `opacity`; configure with `configure_tick`.
- Use it for: rug plots, strip plots, marginal density marks, and dense
  positional displays where individual points would merge into a cloud.
- Compared with `mark_point`: a point is a shape with a data-driven size; a
  tick has a fixed linear length and emphasizes positional density.

### Rect Mark (`mark_rect`)

Represents each row as an **axis-aligned cell**. This is the heatmap geometry:
`x` and `y` position the cell, `color` supplies its value, and the cell size is
derived from the bin layout rather than from arbitrary vertices.

- Per row: one rectangle; rows are never connected.
- Required: `x`, `y` and `color`.
- Continuous axes: charton computes bins, so `mark_rect` can act as a 2-D
  histogram. Categorical axes give one cell per category pair.
- Visuals: `color`, `opacity`, `stroke`, `stroke_width`, `cell_padding`;
  configure with `configure_rect`.
- Use it for: heatmaps, 2-D histograms, correlation matrices and any regular
  grid of values.
- Compared with `mark_polygon`: a rectangle is a *special case* of a polygon,
  but the marks model different data. `mark_rect` is axis-aligned and
  bin/cell-oriented; `mark_polygon` accepts arbitrary rings, including concave
  regions and holes.

### Specialized Marks

- Rule: Used for annotations, ranges and error margins.
- Text: Placing strings directly into the coordinate space.

## From Mark to Geometry: The Renderer

Behind every `Mark` lies a corresponding Renderer. When Charton enters the "Realization" phase (Chapter 3.4), it translates the mark's configuration into physical geometry:

- PointElement: A simple struct containing `x, y, shape, size`.
- PathConfig: one or more subpaths plus fill/stroke properties, used for lines, areas, polygons and multi-ring regions.
- RectConfig: defined by `x, y, width, height`; used by bars, histogram cells, ticks and `mark_rect` heatmap cells.

## Marks and Categorical Stacking

One of Charton's advanced features is how Marks handle Stacking and Grouping.

As seen in the `MarkBar` implementation, when multiple series exist on the same X-coordinate:

- Stacked: The `Y` value of the second mark starts at the `Y` end-point of the first.
- Grouped (Side-by-Side): Marks are offset by a fraction of the category slot so they sit next to each other without manual coordinate calculation. The arithmetic lives in [Position](../concepts/grammar_pipeline.md), shared by bars, boxes, error bars, points and violins.

## Connecting the dots: line, path and polygon

Three geometries connect a sequence of points. They answer two questions
differently:

1. **In what order** are the points joined?
2. Is the shape **closed and filled**?

| Geometry | Order | Grouped by | Closed? | Filled? | ggplot2 | Typical use |
|---|---|---|---|---|---|---|
| `mark_line` | sorted by x | colour | no | no | `geom_line` | time series, trend lines |
| `mark_path` | **row order** | `path_group` | no | no | `geom_path` | trajectory, contour line, edges |
| `mark_polygon` | row order | `path_group` | **yes** | **yes** | `geom_polygon` | violin outline, map, filled ribbon |

Geographic maps use `mark_polygon` with `CoordSystem::Geo`: projection belongs
to the coordinate system, not to a separate mark.

### Open vs closed is the whole difference

Draw the same three points with each geometry and look at the produced path:

```text
mark_path     M p0 L p1 L p2                 (stops at p2; stroked)
mark_polygon  M p0 L p1 L p2 Z               (Z joins p2 back to p0; filled)
```

So a "trajectory" and an "outline" are **not** the same idea, even though both
are drawn from a sequence of points:

* a **trajectory** is an open curve — it has two loose ends and nothing to fill
  ⇒ `mark_path`;
* an **outline** is a closed loop that encloses an area to fill ⇒ `mark_polygon`.

A contour line is the classic trap. A single contour can itself be a closed loop
(around a peak), but you still draw it with `mark_path`: you want the **line**,
not a filled disc. Only a *filled* contour band is a `mark_polygon`.

Two mistakes follow from ignoring the distinction: giving an open curve to
`mark_polygon` adds a straight edge from the last point back to the first; and
giving a line loop to `mark_polygon` fills it, hiding everything inside.

```rust
// A trajectory: open, order preserved, stroked.
chart!(x, y, id)?
    .mark_path()?
    .encode((alt::x("x"), alt::y("y"), alt::path_group("id")))?;

// The same points as a filled region: closed and filled.
chart!(x, y, id)?
    .mark_polygon()?
    .encode((alt::x("x"), alt::y("y"), alt::path_group("id")))?;
```

### The five geometries side by side

`mark_tick` and `mark_rect` are often confused with the three connected
geometries. The first question is not "which shape?" but **"does one row stand
alone, or does it join its neighbours?"**

| Mark | One row becomes | Connects rows? | Order | Grouping | Closed / filled | Size from | Typical use |
|---|---|---|---|---|---|---|---|
| `mark_line` | a point on a series | yes, within a colour group | sorted by x | `color` | open / no fill | data | time series, trends |
| `mark_path` | a vertex of a polyline | yes, within a path group | **row order** | `path_group` (+ subpath) | open / no fill | data | trajectories, contour lines, edges |
| `mark_polygon` | a vertex of a ring | yes, within a path group | **row order** | `path_group` (+ subpath) | **closed / even-odd fill** | data | violins, maps, filled bands |
| `mark_tick` | one short segment | **no** | — | — | independent glyph | **fixed pixels** | rug, strip, marginal density |
| `mark_rect` | one axis-aligned cell | **no** | — | `color` (value) | closed cell / fill | **bin/cell layout** | heatmaps, 2-D histograms |

A practical decision flow:

1. **Does each row stand alone?**
   - Yes → is the mark a fixed-size positional glyph (`mark_tick`) or a cell
     that carries a value (`mark_rect`)?
   - No → the rows form a sequence; continue.
2. **Should the sequence follow x or the data order?**
   - x → `mark_line` (colour groups, sorting, optional smoothing).
   - data order → `mark_path` or `mark_polygon`.
3. **Should the shape be closed and filled?**
   - No → `mark_path`.
   - Yes → `mark_polygon`.

This is also why `mark_polygon` with `fill = none` is not `mark_path`: it still
closes every ring and draws the closing edge. Filling is an independent choice;
closure is what defines the geometry.

### Marks vs backend primitives

The low-level backend reuses fewer primitives than there are marks:

| Mark | Backend primitive |
|---|---|
| `mark_tick`, `mark_rect` | `draw_rect` (plus the shared paint/text machinery) |
| `mark_line`, `mark_path`, `mark_polygon`, `mark_area` | `draw_path` with a topology hint |

Sharing a primitive does not make two marks equivalent. `mark_tick` and
`mark_rect` both draw a rectangle, but one is a fixed-size positional glyph and
the other is a binned value cell. `mark_line` and `mark_path` both draw a path,
but one sorts and groups by colour while the other preserves row order and
groups by `path_group`. The mark is the **data-to-geometry contract**; the
primitive is only the implementation.

## Marks are primitives; charts are compositions

A mark draws the data it is given. It does not know about statistics or
grouping — those belong to earlier stages. Complex chart types are therefore
*compositions*, not new marks. A violin plot is a density statistic, the general
band geometry and the shared polygon mark:

```rust
chart!(iris)?
    .transform_density(
        DensityTransform::new("sepal_length")
            .with_as("sepal_length", "density")
            .with_trim(true),
    )?
    .transform_band(BandTransform::new("sepal_length", "density"))?
    .mark_polygon()?
    .encode((alt::x("x"), alt::y("y"), alt::path_group("path_group")))?
```

Rainclouds and split violins stack more layers (`.and(…)`) on the same parts.
See [The Layer Pipeline](../concepts/grammar_pipeline.md) and
[Violin](../gallery/violin.md).

## Visual Consistency (The Mark Trait)

In `mark.rs`, the `Mark` trait ensures that all geometric primitives share a common interface. This allows the `LayeredChart` to treat a `PointChart` and a `LineChart` identically during the high-level orchestration phase, even though their low-level draw calls are completely different.

## Key Takeaways

- Marks are the "ink" on the page.
- Mark choice changes the narrative of the data (e.g., a Line implies a trend, while a Bar implies a comparison).
- Geometric resolution is the final step where abstract scales are converted into physical shapes (Circles, Rects, Paths).