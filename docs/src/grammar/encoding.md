# Encodings
Encodings are the core of Charton’s declarative visualization system. They determine **how data fields map to visual properties** such as:
- Position (`x`, `y`, `y2`, `theta`)
- Color
- Shape
- Size
- Text labels

Every chart in Charton combines:
1. **A mark** (point, line, bar, arc, rect, etc.)
2. **Encodings** that map data fields to visual channels

This chapter explains all encoding channels, how they work, and provides complete code examples using **mtcars.**

## What Are Encodings?
An encoding assigns a *data field* to a *visual* channel.
```rust
Chart::build(&df)?
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        color("cyl"),
    ))?;
```
This produces a scatter plot:

* **X axis** → horsepower
* **Y axis** → miles per gallon
* **Color** → number of cylinders

## Encoding System Architecture
Every encoding implements the following trait:
```rust
pub trait IntoEncoding {
    fn apply(self, enc: &mut Encoding);
}
```

Users **never** interact with `Encoding` directly.
They simply write:
```rust
.encode((x("A"), y("B"), color("C")))
```

The API supports tuple-based composition of up to **9 encodings.**

## Position Encodings
### X – Horizontal Position

The **X** channel places data along the horizontal axis.

✔ **When to use** `X`

- Continuous values (e.g., `hp`, `mpg`, `disp`)
- Categorical values (`cyl`, `gear`, `carb`)
- Histogram binning
- Log scales

**API**
```rust
x("column_name")
```
**Optional settings**
```rust
x("hp")
    .with_bins(30)
    .with_scale(Scale::Log)
    .with_zero(true)
```
**Example: mtcars horsepower vs mpg**
```rust
let df = load_dataset("mtcars");

Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
    ));
```
Expected: Scatter plot showing `hp` vs `mpg`.

### Y – Vertical Position

The **Y** channel has identical behavior to `X`.

**Example**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("wt"),
        y("mpg"),
    ));
```
Expected: Heavier cars generally have lower mpg.

### Y2 – Second Vertical Coordinate

Used when a mark needs **two vertical positions:**

- Interval bands
- Confidence intervals
- Error bars
- Range rules

**Example: Upper & Lower MPG Bounds**
```rust
Chart::build(&df)
    .mark_area()
    .encode((
        x("hp"),
        y("mpg_low"),
        y2("mpg_high"),
    ));
```
## Angular Position: θ (Theta)
Used in:

- Pie charts
- Donut charts
- Radial bar charts

**Example: Pie chart of cylinders**
```rust
Chart::build(&df)
    .mark_arc()
    .encode((
        theta("count"),
        color("cyl"),
    ));
```
## Color Encoding
Color maps a field to the fill color of a mark.

✔ **When to use**

- Categorical grouping
- Continuous magnitude
- Heatmaps
- Parallel categories

**Example: Color by gears**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        color("gear"),
    ));
```
## Shape Encoding
**Shape – Point Symbol Mapping**

Only applies to **point marks**.

**Available shapes include:**

- Circle
- Square
- Triangle
- Cross
- Diamond
- Star

**Example**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        shape("cyl"),
    ));
```
## Size Encoding
**Size – Radius / Area Encoding**

Used for:

- Bubble plots
- Weighted scatter plots
- Emphasizing magnitude

**Example: Bubble plot with weight**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        size("wt"),
    ));
```
## Opacity Encoding
**Opacity – Transparency**

Used for:

- Reducing overplotting
- Encoding density
- Showing relative uncertainty

**Example: Opacity mapped to horsepower**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("wt"),
        y("mpg"),
        opacity("hp"),
    ));
```
## Text Encoding
**Text – Label Encoding**

Works with:

- Point labels
- Bar labels
- Annotation marks

**Example: Label each point with car model**
```rust
Chart::build(&df)
    .mark_text()
    .encode((
        x("hp"),
        y("mpg"),
        text("model"),
    ));
```
## Stroke Encoding
**Stroke – Outline Color**

Useful when:

- Fill color is already used
- Emphasizing boundaries
- Donut chart outlines

**Example**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        stroke("gear"),
    ));
```
## Stroke Width Encoding
**Stroke Width – Border Thickness**

Used for:

- Highlighting
- Encoding magnitude
- Interval charts

**Example**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        stroke_width("wt"),
    ));
```
## Combined Example: All Encodings

This chart uses eight encodings simultaneously:
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        color("cyl"),
        shape("gear"),
        size("wt"),
        opacity("qsec"),
        stroke("carb"),
        stroke_width("drat"),
    ));
```

Expected:
A rich multi-dimensional visualization of `mtcars`.

## Configuring Encodings (The Intent Pattern)
Charton uses a unified interface where you define your **Intent** (e.g., "I want this column to be on the X-axis using a Log scale"), and the engine handles the **Resolution** (calculating the actual pixel coordinates).

Each encoding (like `X`, `Color`, or `Size`) follows a **Fluent Builder Pattern**. You can refine how the mapping behaves by chaining methods on the encoding object *before* passing it to the chart.

**Position Encoding (`x`, `y`)**

Position encodings control the spatial layout.

```rust
chart.encode((
    x("gdp")
        .with_scale(Scale::Log)      // Use logarithmic transformation
        .with_domain(ScaleDomain::Continuous(0.0, 100.0)) // Force limits
        .with_zero(false)            // Don't force 0.0 into view
        .with_bins(10),              // Aggregate data into 10 bins
    y("population")
))?
```

**Aesthetic Encoding (`shape`, `size`, `opacity`)**

These control the "look" of the marks based on data values.

```rust
chart.encode((
    color("species")
        .with_scale(Scale::Ordinal), // Explicitly treat as categorical
    size("magnitude")
        .with_domain(ScaleDomain::Continuous(1.0, 10.0))
))?
```

**The "Intent vs. Resulution" Architecture**

One of the most powerful features of Charton's encoding system is the separation of **User Intent** and **System Resolution**.

1. **Intent (Inputs)**: When you call `x("price").with_scale(Scale::Log)`, you are defining a **specification**.
2. **Resolution (Outputs)**: During the `build()` phase, Charton's engine scans the data, finds the min/max values, applies your overrides, and "back-fills" a `ResolvedScale`.

**Why this matters:**

Because the `resolved_scale` is stored inside the encoding (often wrapped in an `Arc`), **multiple layers can share the same scale**. If you have a scatter plot and a regression line in the same chart, they will automatically synchronize their axes because they refer to the same resolved intent.

**Avoiding "Mega-Methods"**

Charton avoids polluting the main `Chart` API. Notice that methods like `with_bins` belong to the `X` or `Y` structs, not the `Chart` itself.

- **Incorrect**: `chart.set_x_bins(10)` (Bloats the main API)
- **Correct**: `chart.encode(x("col").with_bins(10))` (Keeps logic namespaced)

This ensures that as Charton adds more complex encoding features (like time-unit formatting or custom color palettes), the top-level API remains clean and easy to navigate.

## Tips & Best Practices

**✔ Use color for major categories**
Examples: `cyl`, `gear`, `carb`.

**✔ Use size sparingly**
Only when magnitude matters.

**✔ Avoid using both color & shape unless required**
Choose one main grouping.

**✔ Use opacity to reduce overplotting**
mtcars has many overlapping data points.

**✔ Avoid encoding more than 5 dimensions**
Human perception becomes overloaded.

## Summary Table
| **Channel**    | **Purpose**          | **Works With** | **Example**         |
| -------------- | -------------------- | -------------- | ---------------------- |
| `x`            | horizontal position  | all marks      | `x("hp")`              |
| `y`            | vertical position    | all marks      | `y("mpg")`             |
| `y2`           | interval upper bound | area, rule     | `y2("high")`           |
| `theta`        | angle (pie/donut)    | arc            | `theta("count")`       |
| `color`        | fill color           | all            | `color("cyl")`         |
| `shape`        | symbol               | point          | `shape("gear")`        |
| `size`         | area/size            | point          | `size("wt")`           |
| `opacity`      | transparency         | point/area     | `opacity("hp")`        |
| `text`         | labels               | text mark      | `text("model")`        |
| `stroke`       | outline color        | point/rect/arc | `stroke("carb")`       |
| `stroke_width` | outline thickness    | all            | `stroke_width("drat")` |
