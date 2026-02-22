## Colors, Palettes, and Colormaps
Charton provides a high-performance color system designed for both static SVG export and real-time WGPU rendering. The system is centered around three core types: `SingleColor`, `ColorPalette`, and `ColorMap`.

### The `SingleColor` Type
`SingleColor` is a lightweight, stack-allocated struct that stores colors as normalized RGBA values ($[0.0, 1.0]$).
- **Memory Efficient**: Implements Copy, avoiding heap allocations.
- **Backend Ready**: Maps directly to GPU vertex buffers while providing on-the-fly CSS string generation for SVGs.
- **Flexible Creation**: Supports CSS strings (Hex, RGB, HSL, Named colors), RGBA arrays, and a special "None" state for transparency.

```rust
// Creation examples
let red = SingleColor::new("#ff0000");           // Hex
let semi_blue = SingleColor::new("rgba(0,0,255,0.5)"); // CSS Functional
let transparent = SingleColor::none();           // Fully transparent
let from_array: SingleColor = [0.0, 1.0, 0.0, 1.0].into(); // Green
```

### Color Control Strategies
**A. Mark-Level Colors (Manual)**

Directly setting a color on a mark. This functions when there are no data-driven color encoding.

```
mark_point()
.configure_point(|p| p.with_point_color("steelblue")) // Accepts any into<SingleColor>
```

**B. Discrete Palettes (Categorical Data)**

When mapping data groups to colors (e.g., different species in a scatter plot), use `ColorPalette`. Charton includes industry-standard palettes from Tableau and ColorBrewer.

|  Palette Type | Variants                      |  Use Case                              |
|---------------|-------------------------------|----------------------------------------|
|**Standard**   |`Tab10`, `Tab20`               |General purpose, clear differentiation. |
|**Qualitative**|`Set1`, `Set2`, `Set3`         |Categorical data with no inherent order.|
|**Stylized**   |`Pastel1`, `Dark2`, `Accent`   |Specific aesthetic requirements.        |

```rust
// Usage: automatically wraps indices if groups exceed palette size
chart.configure_theme(|t| t.with_palette(ColorPalette::Tab20))
```

**C. Continuous Colormaps (Numerical Data)**

For heatmaps or data-driven gradients, use `ColorMap`. These provide smooth transitions based on a normalized value ($0.0 \dots 1.0$).
- **Perceptually Uniform**: `Viridis`, `Inferno`, `Magma`, `Plasma`, `Cividis`. These are mathematically designed to represent data accurately, even for color-blind viewers or when printed in grayscale.
- **Sequential**: `Blues`, `Reds`, `YlGnBu` (Yellow-Green-Blue), etc.
- **Specialized**: `Rainbow`, `Jet`, `Cool`, `Hot`.

```rust
// Usage: maps numerical intensity to a color gradient
chart.configure_theme(|t| t.with_color_map(ColorMap::Viridis))
```
