# Violin

A violin shows the **density** of a numeric value as a symmetric outline. There
is no violin mark and no violin-specific transform — it is a short recipe over
the primitives:

```text
transform_density(trim = true)        // one curve per category (and group)
  → a symmetric outline               // mark_area stack "center"/"mirror", or transform_band
  → coord_flip                        // stand the value axis upright
```

Every variant on this page is that same core with **one thing changed**: a
grouping, a facet, a split, or an extra layer. Read the first recipe, then diff
the rest — that is the grammar in action.

## Violin (single)

<img src="../images/violin.svg" width="500">

Estimate the density, draw it mirrored around zero, and stand the value axis up.

```rust
{{#include ../../../examples/violin.rs}}
```

## Grouped and faceted violins

<img src="../images/grouped_violin.svg" width="500">
<img src="../images/faceted_violin.svg" width="500">

Two industry layouts from the same density transform:

- **Faceted** (Vega-Lite / Altair style): draw the mirrored area and let `facet`
  give each group its own panel.
- **Dodged** (ggplot2 style): group the density by `(category, group)`, turn the
  curve into a symmetric `transform_band` polygon, and let a `Position` dodge the
  lanes side by side.

```rust
{{#include ../../../examples/grouped_violin.rs}}
```

## Split violin

<img src="../images/split_violin.svg" width="500">

One centre line per category, the first group growing right and the second left:
`transform_band(…).with_split(true)`.

```rust
{{#include ../../../examples/split_violin.rs}}
```

## Raincloud

<img src="../images/raincloud.svg" width="500">

Three views of the same distribution, stacked with `.and(…)`: the violin
outline, an inner inter-quartile box from `transform_quantile_box`, and every
observation as a jittered point layer.

```rust
{{#include ../../../examples/raincloud.rs}}
```

## Choosing a layout

| Variant | Use it when |
| --- | --- |
| Single violin | one distribution |
| Faceted | compare a few groups, each with room to breathe |
| Dodged | compare many groups within one category on a shared axis |
| Split | exactly two groups, to save horizontal space |
| Raincloud | the sample is small enough that showing every point matters |

## See also

- [Statistical Distributions](statistics.md) — the density/band machinery and
  tuning the shape (`BandScale`, bandwidth, kernel).
- [Box & Violin Combinations](box_violin_charts.md) — overlaying a box on a
  dodged violin.
- Primitives: `transform_density`, `transform_band`,
  `transform_quantile_box`, `mark_polygon`, `mark_area` — see
  [Marks & Geometries](../grammar/marks.md).
