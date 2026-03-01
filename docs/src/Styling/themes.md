# Styling and Themes

A chart is not complete when it merely *works*.

A great chart communicates clearly, resonates visually, and fits seamlessly into its context—whether that context is a publication, dashboard, presentation slide, or internal report.

Charton provides a rich and structured styling system that allows you to control every aspect of chart appearance: themes, axes, fonts, sizes, colors, spacing, and layout.

Styling in Charton is **declarative, layered, and predictable**, following design principles familiar from Altair (Vega-Lite) and ggplot2.

## Styling Model and Precedence
Charton follows a **three-level styling model**:

1. **Theme-level styling** (`Theme`)
Defines global defaults such as colors, fonts, paddings, and visual identity.
2. **Chart-level styling** (`LayeredChart` / common builders)
Adjusts layout, axes, domains, labels, and global chart properties.
3. **Mark-level styling** (`mark_*`)
Controls the appearance of individual visual elements such as color, size, and shape.

**Styling Precedence**

When multiple levels specify the same visual property, Charton resolves them in the following order:

> **Mark-level overrides** → **Chart-level overrides** → **Theme defaults**

This guarantees that:
- Themes establish a consistent visual baseline
- Charts can adapt styling to a specific figure
- Marks retain precise, local control when needed

## Themes and Presets
Themes define the overall visual identity of a chart: colors, fonts, axis strokes, paddings, and spacing. In Charton, themes are represented by the `Theme` struct and applied globally:
```rust
chart.with_theme(Theme::default())
```
**Built-in Themes**

Charton provides several built-in themes:

- **Default** — light theme suitable for most use cases
- **Minimal** — reduced visual noise, thin strokes, no grid emphasis (To be implemented)
- **Classic** — thicker axes, Matplotlib-style appearance (To be implemented)
- **Dark** — optimized for dashboards and dark backgrounds (To be implemented)

Example:
```rust
let chart = LayeredChart::new()
    .with_theme(Theme::default());
```
**Customizing Theme Fields**

All theme fields are manually adjustable by overridding at the chart level.
```rust
let chart = LayeredChart::new()
    .with_theme(Theme::default())
    .with_label_font_size(10);
```

## Chart-Level Styling: Axes and Layout
Chart-level styling is applied via shared builder methods and affects **all layers**.

**Axis Domains**

Override automatic domain inference:
```rust
chart
    .with_x_domain(0.0, 10.0)
    .with_y_domain_min(5.0);
```
**Axis Labels**
```
chart
    .with_x_label("Time (s)")
    .with_y_label("Intensity");
```
Padding and rotation:
```rust
chart
    .with_x_label_padding(25.0)
    .with_x_label_angle(-45.0);
```
**Tick Values and Labels**

**Continuous axis:**
```rust
chart.with_x_tick_values(vec![0.0, 2.0, 4.0, 6.0, 8.0]);
```
**Discrete axis:**
```rust
chart.with_x_tick_labels(vec!["A", "B", "C"]);
```
Rotate tick labels to avoid overlap:
```rust
chart.with_x_tick_label_angle(45.0);
```
Chart-level axis settings always override theme defaults.

### Technical Performance Note
Unlike many visualization libraries that parse strings at render-time, Charton’s color system:
1. **Pre-calculates** all Palette/Map values into normalized float arrays.
2. Uses **linear interpolation** for continuous maps, ensuring zero string-processing overhead during GPU draw calls.
3. **Clamps** all inputs to valid ranges $[0.0, 1.0]$ to prevent rendering artifacts.

## Shapes and Sizes (Mark-Level Styling)
Shape and size are **mark-specific properties** and never affect other layers.

**Point Shape and Size**
```rust
let point = mark_point()
    .with_point_shape(PointShape::Circle)
    .with_point_size(60.0);
```
**Data-Driven Shape and Size**
```rust
mark_point()
    .encode((
        x("time"),
        y("value"),
        shape("group")
    ))?;
```
or
```rust
mark_point()
    .encode((
        x("wt"),
        y("mpg"),
        size("cyl")
    ))?;
```
Encoding-driven shape and size always use the chart defaults.

## Font and Text Styling
Typography plays a major role in readability. Font and text styles currently inherit theme settings but can be overridden at the chart level.

Rendering text consistently across different platforms is a major challenge. Charton solves this with a **Double-Safety** strategy, though the behavior differs between PNG and SVG.

**1. The Built-in "Emergency" Font (For PNG)**

Unlike many libraries, Charton **embeds** the **Inter** font directly into its binary (`include_bytes!`).
- **Zero-Dependency**: You don't need to install font packages in minimal Docker containers or headless servers.
- **Guaranteed Rendering**: When exporting to **PNG**, text is "**baked-in**" (rasterized) into pixels. If your requested system fonts are missing, Charton falls back to the internal Inter font. This ensures the PNG looks identical on every machine.

**2. The SVG Limitation: "Instructional" Rendering**

It is important to note that **SVG works differently**:
- **Browser-Dependent**: When exporting to **SVG**, Charton does not embed the font data. Instead, it writes a "font-family" instruction into the code.
- **The Result**: The final look depends on the **viewer's browser**. If the viewer doesn't have your specified font, their browser will use its own default.

**3. The Default Font Stack**

To maximize compatibility for both formats, Charton traverses a prioritized **Font Stack**:
- **Modern Sans-Serifs**: `Inter`, `-apple-system` (macOS), `BlinkMacSystemFont`.
- **Windows Standards**: `'Segoe UI'`, `Roboto`, `Arial`.
- **Linux Essentials**: `Ubuntu`, `Cantarell`, `'Noto Sans'`.
- **System Generic**: `sans-serif` (In PNG, this maps to our embedded Inter).

**4. Overriding at the Chart Level**

You can define your own stack using a comma-separated string:

```rust
chart
    .with_title("Global Genomics Report")
    // Falls back to Helvetica, then Arial, then the internal "sans-serif"
    .with_title_font_family("CustomBrandFont, Helvetica, Arial, sans-serif")
    .with_title_font_size(24);
```

## Chart Dimensions, Margins, and Background
**Dimensions**
```rust
chart.with_size(800, 600);
```
Larger sizes improve readability for dense charts.

**Margins**

Margins are expressed as **fractions of total size**:
```rust
chart
    .with_left_margin(0.15)
    .with_right_margin(0.10)
    .with_top_margin(0.10)
    .with_bottom_margin(0.15);
```

**Background and Legend**
```rust
chart.with_background("#fafafa");

chart
    .with_legend(true)
    .with_legend_title("Experimental Groups");
```
Legend appearance is influenced by the active theme.

## Complete Example: Before and After Styling
**Basic Chart (Default Styling)**
```rust
let chart = Chart::build(&df)?
    .mark_point()
    .encode((x("x"), y("y")))?
    .into_layered();
```
**Styled Chart**
```rust
let chart = Chart::build(&df)?
    .mark_point()
    .encode((x("x"), y("y")))?
    .into_layered();

chart
    .with_theme(Theme::default())
    .with_title("Styled Scatter Plot")
    .with_x_label("X Value")
    .with_y_label("Y Value")
    .with_size(800, 600)
    .with_background("#ffffff")
    .save("chart.svg")?;
```
This demonstrates how themes, chart-level settings, and mark-level styling compose naturally.

**Style Resolution Summary**

| **Level** | **Scope** | **Typical Usage**            |
| ----- | --------- | -------------------------------- |
| Theme | Global    | Visual identity, fonts           |
| Chart | Per chart | Axes, layout, labels, domains    |
| Mark  | Per layer | Color, size, shape               |

Charton’s styling system is designed to be:
- **Declarative** — no imperative styling logic
- **Layer-aware** — global defaults with local overrides
- **Consistent** — predictable resolution rules

This allows users to create publication-quality visualizations with minimal effort, while still enabling deep customization when required.
