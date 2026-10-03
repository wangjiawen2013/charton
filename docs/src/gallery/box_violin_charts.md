# Box & Violin Combinations

The violin chapter built a single violin from a density stat, a position and a
polygon. Because those parts are independent, they can be rearranged and stacked
into the richer pictures researchers commonly need. Nothing below adds a new
mark — every picture is a stack of ordinary layers.

> For the complete, compiled recipes (single → grouped → faceted → split →
raincloud) start at the [Violin cookbook page](violin.md). This page goes deeper
on stacking a violin with a box.

## Raincloud: violin + box + points

A *raincloud* shows the same distribution three ways: a density outline, a box
plot for the quartiles, and the raw observations. Each view is a layer, and all
of them share one scale so they line up automatically.

<img src="../images/raincloud.svg" width="500">

```rust
{{#include ../../../examples/raincloud.rs}}
```

## Split violin

A split violin contrasts two groups inside one outline: the first group grows to
the right of the category centre, the second to the left. Density groups by the
two fields, and the band geometry is asked for `with_split(true)`; colour the
result by group and the two halves read as a single violin split down the
middle.

<img src="../images/split_violin.svg" width="500">

```rust
{{#include ../../../examples/split_violin.rs}}
```

## Overlaying a box on a normally-dodged violin

`transform_quantile_box` computes the quartiles per `(category, group)` cell, so
any overlay can read them directly. For a dodged (side-by-side) violin, layer it
on top exactly as in the raincloud, but keep the same `Position::Dodge` on the
band and on the box so the boxes follow their violins.

## Why this is better than a dedicated "violin mark"

Every combination above — raincloud, split, overlay, grouped, faceted — would
need bespoke branching inside a single `MarkViolin`. Building from a density
stat plus a polygon instead means:

* the polygon renderer is shared with maps and custom shapes;
* new combinations are new layers, not new renderers;
* every rendering backend (SVG, PNG, PDF, GPU) already knows how to draw the
  parts.

## See also

* [Statistical Distributions](statistics.md) — the basic violin and box plots.
* [The Layer Pipeline](../concepts/grammar_pipeline.md) — the stat/position/geom
  model behind these examples.
* `examples/raincloud.rs`, `examples/split_violin.rs`, `examples/violin.rs`,
  `examples/grouped_violin.rs`.
