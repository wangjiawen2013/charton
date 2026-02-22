# Marks
Marks are the fundamental building blocks of Charton. A *mark* is any visible graphical primitive—points, lines, bars, areas, rectangles, arcs, text, boxplots, rules, histograms, and more.

Every chart in Charton is created by:
**1.** Constructing a base chart using `Chart::build()`.
**2.** Selecting a mark type (e.g., `mark_point()`, `mark_line()`).
**3.** Adding encodings that map data fields to visual properties.

Understanding marks is essential because **most visual expressiveness comes from combining marks with encodings.**

## What Is a Mark?
In Charton, a mark is an object that implements the core trait:
```rust
pub trait Mark: Clone {
    fn mark_type(&self) -> &'static str;

    fn stroke(&self) -> Option<&SingleColor> { None }
    fn shape(&self) -> PointShape { PointShape::Circle }
    fn opacity(&self) -> f64 { 1.0 }
}
```
**Key Properties**
| **Property**| **Meaning**       | **Provided by Trait** |
| ----------- | ----------------- | ----------------- |
| `mark_type` | Unique identifier | required          |
| `stroke`    | Outline color     | default: none     |
| `shape`     | Point shape       | default: circle   |
| `opacity`   | Transparency      | default: 1.0      |

## How Marks Work in Charton
A typical Charton chart:
```rust
Chart::build(&df)?
    .mark_point()
    .encode((
        x("x"),
        y("y")
    ))?
```
**Flow of Rendering**

**1.** `mark_point()` creates a `MarkPoint` object.

**2.** Encodings specify how data fields map to visual properties.

**3.** Renderer merges:
- mark defaults
- overriding encoding mappings
- automatic palettes

**4.** The final SVG/PNG is generated.

**Declarative Design Philosophy**

Charton follows an Altair-style declarative model:

> **If an encoding exists → encoding overrides mark defaults.**

> **If an encoding does not exist → use the mark’s own default appearance.**

This gives you:
- Short expressions for common charts
- Fine-grained control when needed

## Point Mark
MarkPoint draws scattered points.

**Struct (simplified)**
```rust
pub struct MarkPoint {
    pub color: Option<SingleColor>,
    pub shape: PointShape,
    pub size: f64,
    pub opacity: f64,
    pub stroke: Option<SingleColor>,
    pub stroke_width: f64,
}
```
**Use Cases**
- Scatter plots
- Bubble charts
- Highlighting specific points
- Overlaying markers on other marks

**Correct Example**
```rust
Chart::build(&df)?
    .mark_point()
    .encode((
        x("sepal_length"),
        y("sepal_width"),
        color("species"),
        size("petal_length")
    ))?
```
## Line Mark
MarkLine draws connected lines.

**Highlights**
- Supports LOESS smoothing
- Supports interpolation

**Struct**
```rust
pub struct MarkLine {
    pub color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
    pub use_loess: bool,
    pub loess_bandwidth: f64,
    pub interpolation: PathInterpolation,
}
```
**Example**
```rust
Chart::build(&df)?
    .mark_line().transform_loess(0.3)
    .encode((
        x("data"),
        y("value"),
        color("category")
    ))?
```
## Bar Mark
A bar mark visualizes categorical comparisons.

**Struct**
```rust
pub struct MarkBar {
    pub color: Option<SingleColor>,
    pub opacity: f64,
    pub stroke: Option<SingleColor>,
    pub stroke_width: f64,
    pub width: f64,
    pub spacing: f64,
    pub span: f64,
}
```
**Use Cases**
- Vertical bars
- Grouped bars
- Stacked bars
- Horizontal bars

**Example**
```rust
Chart::build(&df)?
    .mark_bar()
    .encode((
        x("type"),
        y("value"),
    ))?
```
## Area Mark
Area marks fill the area under a line.

**Example**
```rust
Chart::build(&df)?
    .mark_area()
    .encode((
        x("time"),
        y("value"),
        color("group")
    ))?
```
## Arc Mark (Pie/Donut)
Arc marks draw circular segments.

**Example (donut)**
```rust
Chart::build(&df)?
    .mark_arc()  // Use arc mark for pie charts
    .encode((
        theta("value"),  // theta encoding for pie slices
        color("category"),  // color encoding for different segments
    ))?
    .with_inner_radius_ratio(0.5) // Creates a donut chart
```
## Rect Mark (Heatmap)
Used for heatmaps and 2D densities.

**Example**
```rust
Chart::build(&df)?
    .mark_rect()
    .encode((
        x("x"),
        y("y"),
        color("value"),
    ))?
```
## Boxplot Mark
Visualizes statistical distributions.

**Example**
```rust
Chart::build(&df_melted)?
    .mark_boxplot()
    .encode((
        x("variable"),
        y("value"),
        color("species")
    ))?
```
## ErrorBar Mark
Represents uncertainty intervals.

**Example**
```rust
// Create error bar chart using transform_calculate to add min/max values
Chart::build(&df)?
    // Use transform_calculate to create ymin and ymax columns based on fixed std values
    .transform_calculate(
        (col("value") - col("value_std")).alias("value_min"),  // ymin = y - std
        (col("value") + col("value_std")).alias("value_max")   // ymax = y + std
    )?
    .mark_errorbar()
    .encode((
        x("type"),
        y("value_min"),
        y2("value_max")
    ))?
```
## Histogram Mark
Internally used to draw histogram bins.

**Example**
```rust
Chart::build(&df)?
    .mark_hist()
    .encode((
        x("value"),
        y("count").with_normalize(true),
        color("variable")
    ))?
```
## Rule Mark
Draws reference lines.

**Example**
```rust
Chart::build(&df)?
    .mark_rule()
    .encode((
        x("x"),
        y("y"),
        y2("y2"),
        color("color"),
    ))?
```
## Text Mark
Places textual annotations.

**Example**
```rust
Chart::build(&df)?
    .mark_text().with_text_size(16.0)
    .encode((
        x("GDP"),
        y("Population"),
        text("Country"),
        color("Continent"),
    ))?
```

## Advanced Mark Configuration (Mark Styling)
While **Encodings** link data to visual properties, you often need to set **fixed** visual constants for a specific layer—for example, making all points red regardless of data, or adding a specific stroke to a line.

Charton provides a **Closure-based Configuration** for every mark type. This is the highest level of styling precedence.

**Why Closures?**

1. **Type Safety**: You only see methods relevant to that specific mark (e.g., you can't set "stroke width" on a property that doesn't support it).
2. **Fluent Chaining**: You can update multiple properties in a single, readable block.
3. **Encapsulation**: Mark properties remain private to the rendering engine, accessible only through this controlled interface.
4. **Namespace Hygience & API Scalability**

Charton’s closure-based design solves two major architectural challenges:

- **Namespace Isolation**: It prevents naming collisions between different mark types. For example, both `MarkPoint` and `MarkText` can expose a `.with_size()` method without ambiguity, as they exist only within their respective closures.
- **Avoiding API Bloat**: It prevents the main `Chart` and `LayeredChart` structs from becoming "mega-structs" with hundreds of prefixed methods (like `.with_point_shape()` or `.with_bar_padding()`). This keeps the top-level API clean and ensures that the IDE's auto-completion remains helpful and intuitive.

**The `configure_xxx` Pattern**
Each mark has a corresponding configuration method (e.g., `configure_point`, `configure_bar`). These methods allow you to "reach inside" the mark and tweak its properties fluently.

```rust
Chart::build(&df)?
    .mark_point()
    // This closure provides direct access to the MarkPoint struct
    .configure_point(|m| {
        m.with_color("steelblue")
         .with_size(10.0)
         .with_stroke("white")
         .with_stroke_width(1.5)
         .with_opacity(0.8)
    })
    .encode((x("time"), y("value")))?
```

**Precedence: Style vs. Encoding**

It is important to remember the **Override Rule**:
1. **Mark Closures** (`configure_xxx`) take absolute priority.
2. **Encodings** (`encode`) come second.
3. **Theme Defaults** are the fallback.

**Note**: If you set `m.with_color("red")` in a closure, any `color("column_name")` mapping in your encoding will be ignored for that specific property.

**Common Configuration Methods**

|Mark Type|Config Method|Key Properties to Tweak|
|---------|-------------|-----------------------|
|Point|`configure_point`|`shape`,`size`,`stroke`,`stroke_width`|
|Line|`configure_line`|`stroke_width`,`interpolate`,`dash_array`|
|Bar|`configure_bar`|`width`,`spacing`,`corner_radius`|
|Text|`configure_text`|`font_size`,`angle`,`align`,`baseline`|

## Summary
* Each mark defines a visual primitive.
* Marks are combined with *encodings* to bind data to graphics.
* Charton uses a declarative approach:
    * Encodings override mark defaults.
    * Palette and scales are automatically applied.
* By choosing the correct mark, you control how data is represented.