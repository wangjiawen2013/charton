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

### Line Mark (`mark_line`)

Connects data points in a specific order (usually by the X-axis) to show trends.

- Connectivity: Unlike points, the Line mark treats a sequence of rows as a single continuous path.
- Visuals: Focuses on `stroke_width` and `color`.

### Bar Mark (`mark_bar`)

Represents data as rectangles extending from a baseline.

- Physicality: Bars have "width." Charton calculates this width based on the `CoordLayout` (Chapter 1.4) to ensure bars don't overlap unless intended.
- Intervals: Uses `X`, `Y` (height), and sometimes `Y2` (for ranged bars).

### Area Mark (`mark_area`)

Similar to a line but filled between a baseline (Y2) and the data value (Y).

- Topology: Highlighting the volume between two series or between a series and the zero-axis.

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
- Grouped (Side-by-Side): The `X` position is offset by a fraction of the "Slot width," ensuring bars are placed next to each other without manual coordinate calculation.

## Visual Consistency (The Mark Trait)

In `mark.rs`, the `Mark` trait ensures that all geometric primitives share a common interface. This allows the `LayeredChart` to treat a `PointChart` and a `LineChart` identically during the high-level orchestration phase, even though their low-level draw calls are completely different.

## Key Takeaways

- Marks are the "ink" on the page.
- Mark choice changes the narrative of the data (e.g., a Line implies a trend, while a Bar implies a comparison).
- Geometric resolution is the final step where abstract scales are converted into physical shapes (Circles, Rects, Paths).