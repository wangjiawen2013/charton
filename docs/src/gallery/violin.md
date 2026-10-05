# Violin

A violin shows the **density** of a numeric value as a symmetric outline.

`mark_violin` is the convenient way to draw one. It is a **composite** mark:
behind the scenes it expands into the ordinary recipe

```text
transform_density(trim = true)        // one curve per category (and group)
  → a symmetric outline               // transform_band polygon
  → the shared polygon renderer
```

so nothing is hidden that you cannot write yourself. Each section below first
shows the mark, then — for anyone who wants to recombine the pieces — how the
same picture is built from the primitives. Read the first recipe, then diff the
rest: that is the grammar in action. The curve itself (bandwidth, kernel, trim)
is described in [1-D Density](density_1d.md).

## Violin (single)

<img src="../images/violin.svg" width="500">

Read the value from `y`; the mark estimates its density and draws the symmetric
outline.

```rust
{{#include ../../../examples/violin.rs}}
```

### Built from the primitives

The same picture without `mark_violin`: estimate the density, mirror it around
zero, and stand the value axis up.

```rust
{{#include ../../../examples/violin_manual.rs}}
```

`"center"` is Charton's name for Vega-Lite's `stack: "center"`: each density curve
is drawn from `-density / 2` to `+density / 2`. `mark_violin` instead uses the
general `transform_band` geometry, which bakes the same mirrored outline — and its
lane placement — into a polygon.

## Grouped and faceted violins

<img src="../images/grouped_violin.svg" width="500">
<img src="../images/faceted_violin.svg" width="500">

Two industry layouts, both one call to `mark_violin` plus the encodings:

- **Faceted** (Vega-Lite / Altair style): read only `y` and `facet` by the group,
  so each panel gets its own violin.
- **Dodged** (ggplot2 style): map `x` to the category and `color` to the group;
  the mark dodges the lanes side by side. The inner box is an ordinary
  `transform_quantile_box` layer, drawn on top with `.and(…)`.

```rust
{{#include ../../../examples/grouped_violin.rs}}
```

When a category used for **position** is missing (the penguins `Sex` column has a
few), that row is dropped, exactly as ggplot2 and Altair would — no `null` violin
appears. A missing **group** (the colour/lane) behaves differently: it is kept as
a grey `NA` lane. See [Missing Values & Gaps](../concepts/missing_values.md).

## Split violin

<img src="../images/split_violin.svg" width="500">

One centre line per category, the first group growing right and the second left:
`mark_violin` with `configure_violin(…).with_split(true)`. Colour the result by
group and the two halves read as a single violin split down the middle.

```rust
{{#include ../../../examples/split_violin.rs}}
```

This is the general `transform_band(…).with_split(true)` geometry underneath: the
two groups share the centre line instead of sitting in separate lanes.

## Raincloud

<img src="../images/raincloud.svg" width="500">

Three views of the same distribution, stacked with `.and(…)`: the violin
outline, an inner inter-quartile box from `transform_quantile_box`, and every
observation as a jittered point layer.

```rust
{{#include ../../../examples/raincloud.rs}}
```

## Tuning the violin

Shape options live in [1-D Density](density_1d.md); these control the mark and
its lanes:

| Method | Meaning |
|---|---|
| `configure_violin(…).with_bandwidth(BandwidthType::Silverman)` | smoothing rule (Scott by default) |
| `.with_trim(true)` | stop each curve at its own data range (the default for violins) |
| `.with_split(true)` | draw two groups as one split violin |
| `.with_scale(BandScale::PerGroup)` | every band has the same maximum width (default) |
| `.with_scale(BandScale::Global)` | bands share one scale, so a denser group looks wider |
| `.with_scale(BandScale::Raw)` | use the width column as it stands |
| `.with_width(0.5)` | maximum width of a single band (matches the box plot) |
| `.with_span(0.7)` | total width of a category's group (matches the box plot and point marks) |
| `.with_color(…)`, `.with_opacity(…)`, `.with_stroke(…)` | the outline's visual style |

## How `mark_violin` is built

A dedicated mark makes the common case one line, while the grammar keeps the hard
cases expressible. Because the mark expands into ordinary parts:

* the polygon renderer is shared with maps and custom shapes;
* new combinations are new layers, not new renderers;
* every rendering backend (SVG, PNG, PDF, GPU) already knows how to draw the
  parts.

Anything the mark does not expose is still reachable with `transform_density`,
`transform_band` / `transform_quantile_box`, and the basic marks.

## Choosing a layout

| Variant | Use it when |
| --- | --- |
| Single violin | one distribution |
| Faceted | compare a few groups, each with room to breathe |
| Dodged | compare many groups within one category on a shared axis |
| Split | exactly two groups, to save horizontal space |
| Raincloud | the sample is small enough that showing every point matters |

## See also

- [1-D Density](density_1d.md) — the KDE and its tuning (bandwidth, kernel, trim).
- [Box Plots](box_plot.md) — the inner quantile box and the box-plot mark.
- [Transforms & Columns](../grammar/transforms.md) — which columns each
  transform reads and emits (`density`, `path_group`, `level`, …).
- Primitives: `transform_density`, `transform_band`, `transform_quantile_box`,
  `mark_polygon`, `mark_area` — see [Marks & Geometries](../grammar/marks.md).
