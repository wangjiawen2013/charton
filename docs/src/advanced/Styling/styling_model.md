# The Hybrid Configuration Pattern
Charton utilizes a **Hybrid Configuration Pattern** to manage the visual complexity of a chart. This design balances "Global Consistency" with "Granular Control" by providing two distinct pathways for customization.

### Direct Overrides (Structural Intent)
On the LayeredChart level, certain properties are exposed via direct builder methods. These typically represent **structural or semantic** changes that relate to how data is perceived.

- **Mechanism**: These methods store values in `Option<T>` fields within the `LayeredChart` struct.
- **Logic**: If a field is `Some`, it acts as a **Hard Override**, bypassing the Theme entirely.
- **Best for**: Axis labels, domain limits (min/max), margins, and coordinate flips.

```rust
let lc = LayeredChart::new()
    .with_x_label("Engine Displacement (L)") // Semantic Override
    .with_y_domain(0.0, 50.0)                // Scale Override
    .with_left_margin(0.2);                  // Layout Override
```

### Fluent Theme Closures (Visual Identity)
For deep aesthetic constants—such as font stacks, tick lengths, grid colors, and legend spacing—Charton uses a **Closure-based Injection** via `configure_theme`.

- **Mechanism**: The `configure_theme<F>(self, f: F)` method passes the current `Theme` into a closure and re-assigns the result.
- **Logic**: This allows you to perform "In-place mutations" on tens of fine-grained parameters without manual re-instantiation.
- **Best for**: Typography, color palettes, stroke widths, and precise legend positioning.

```rust
chart.configure_theme(|t| {
    t.with_background_color("#ffffff")
     .with_title_size(22.0)
     .with_tick_length(10.0)
     .with_legend_item_v_gap(5.0) // Fine-tuning vertical spacing
});
```

### Style Resolution & Precedence
When the renderer decides how to draw a specific element, it follows a strict **Waterfall of Authority**. Understanding this hierarchy is key to troubleshooting "why my style isn't applying."

| Level| Component | Precedence | Responsibility|
| --- | --- | --- | --- |
|Level 1|Mark Closures|🔥 Highest|Local overrides for specific layers (e.g., configure_point)|
|Level 2|Encoding|📊 High|Data-driven mappings (e.g., color(""species""))|
|Level 3|Chart Overrides|📐 Medium|Explicit structural tweaks (e.g., with_x_label)|
|Level 4|Theme Settings|🎨 Baseline|Global visual identity and fallbacks|

**The "Data vs. Visuals" Rule of Thumb**

To keep your code clean and maintainable, follow this principle:
- Use **LayeredChart Methods** for **Data-related** context (What are we looking at? What is the range?).
- Use **Theme Closures** for **Design-related** context (What is the brand color? How thick are the lines?).

### Technical Advantages
1. **Immutability & Safety**: By using the `mut self -> Self` pattern, Charton ensures that styling is a predictable transformation of state, preventing side effects in multi-layered charts.
2. **Explicit Intent**: By separating `with_x_label` from the general theme, Charton makes it obvious that a specific chart has custom data-context that should not be lost even if the theme is swapped globally.
3. **Performance**: Style resolution happens at the "specification" stage. Once the chart is compiled into a `ChartSpec`, the rendering engine uses pre-calculated constants, ensuring zero overhead during the draw loop.