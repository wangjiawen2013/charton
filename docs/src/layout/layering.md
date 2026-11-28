# Layering Grammar

A layer is one `Chart<T>`: a dataset, a mark and the encodings that feed it.
Layering stacks those charts on one coordinate system so they can be read
together — a trend line over a scatter, error bars over a bar.

```rust
use charton::prelude::*;

let points = chart!(age, height)?
    .mark_point()?
    .encode((alt::x("age"), alt::y("height")))?;

let trend = chart!(age, predicted)?
    .mark_line()?
    .encode((alt::x("age"), alt::y("predicted")))?;

let chart = points.and(trend);
chart.save("layered.svg")?;
```

`and` is the whole composition operator. The left chart is drawn first, the right
one on top; a chain `a.and(b).and(c)` keeps that order.

## What is shared

By default the layers share one coordinate system and one scale per channel. If a
point layer and a line layer both map `age` to `x`, they are placed on the same
axis and the same domain. That is what makes a layered chart a single figure
instead of two pictures in a trench coat.

The scales can be overridden on the combined chart: `with_x_domain`,
`with_y_domain`, `coord_flip`, `with_coord`, `with_size`. A layer still owns its
dataset, mark and style, so two layers may read different tables.

Guides unify too: if both layers colour by the same field, the chart draws one
legend for that field, not one per layer.

## What stays local

- **Data.** Each layer keeps its own `Dataset`; there is no join and no shared
  column requirement beyond the fields its own encoding names.
- **Mark and style.** `mark_point` and `mark_line` keep their own sizes, colours
  and transforms.
- **Transforms.** A layer's statistics run on that layer's rows.

## When layering is the wrong tool

- **Several panels** are a facet, not a layer: use `facet` when the split is by a
  field and every panel has the same chart.
- **Several pages** are a composition: use `Composition` when the views sit in a
  grid and may have independent scales. See
  [Composition](composition.md).
- **Nested layers** are refused by the document dialect (`layer` inside a layer).
  Flatten them; the drawing order is the list order.

## See also

- [Multi-View: Faceting](views.md) — one chart, several panels, shared or free
  scales.
- [Composition](composition.md) — independent views on one page.
- [The JSON Spec: layer](../spec/lowering-map.md#layer) — the same rule as a
  document.
