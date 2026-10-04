# Marks & Geometries

While Encodings and Scales define the mathematical relationship between data and space, the Mark is the physical manifestation of that relationship. A Mark is the geometric primitive used to represent a data point or a set of data points.

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

Connects data points in a specific order (usually by the X-axis) to show trends.

- Connectivity: Unlike points, the Line mark treats a sequence of rows as a single continuous path. A missing `x` or `y` breaks the path, so a hole in the data shows as a gap rather than a line drawn across it. See [Missing Values & Gaps](../concepts/missing_values.md).
- Visuals: Focuses on `stroke_width` and `color`.

### Path Mark (`mark_path`)

Connects rows **in row order** without sorting and without closing the shape, and
groups them by `path_group`. This is the "follow the data" curve: trajectories,
contour lines, network edges, parallel coordinates. See
[Connecting the dots](#connecting-the-dots-line-path-and-polygon) for how it
differs from `mark_line` and `mark_polygon`.

### Polygon Mark (`mark_polygon`)

The closed, filled form of the same geometry: rows sharing a `path_group` are
connected in order, the loop is closed and the interior is painted. Violin
outlines, map regions and hand-built filled ribbons use it (`mark_geoshape` is
the geographic name). It fills the polygon you give it; it does not extract the
region between two contour levels.

### Bar Mark (`mark_bar`)

Represents data as rectangles extending from a baseline.

- Physicality: Bars have "width." Charton calculates this width based on the `CoordLayout` (Chapter 1.4) to ensure bars don't overlap unless intended.
- Intervals: Uses `X`, `Y` (height), and sometimes `Y2` (for ranged bars).

### Area Mark (`mark_area`)

Similar to a line but filled between a baseline (Y2) and the data value (Y).

- Topology: Highlighting the volume between two series or between a series and the zero-axis.
- Gaps: A missing `x` or `y` opens the area at that point instead of bridging it. See [Missing Values & Gaps](../concepts/missing_values.md).

### Specialized Marks

- Rule & Tick: Used for annotations or error margins.
- Rect: Drawing arbitrary rectangles based on coordinate pairs.
- Text: Placing strings directly into the coordinate space.

## From Mark to Geometry: The Renderer

Behind every `Mark` lies a corresponding Renderer. When Charton enters the "Realization" phase (Chapter 3.4), it translates the mark's configuration into physical geometry:

- PointElement: A simple struct containing `x, y, shape, size`.
- PathConfig: A collection of points and stroke properties used for Lines and Areas.
- RectConfig: Defined by `x, y, width, height` for Bars and Histograms.

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

`mark_geoshape` is `mark_polygon` under a geographic name; they are the same
renderer.

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

Two mistakes follow from ignoring the flag: giving an open curve to
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
[Box & Violin Combinations](../gallery/box_violin_charts.md).

## Visual Consistency (The Mark Trait)

In `mark.rs`, the `Mark` trait ensures that all geometric primitives share a common interface. This allows the `LayeredChart` to treat a `PointChart` and a `LineChart` identically during the high-level orchestration phase, even though their low-level draw calls are completely different.

## Key Takeaways

- Marks are the "ink" on the page.
- Mark choice changes the narrative of the data (e.g., a Line implies a trend, while a Bar implies a comparison).
- Geometric resolution is the final step where abstract scales are converted into physical shapes (Circles, Rects, Paths).