# Box Plots

A box plot summarises a numeric column with its five-number summary — the
minimum, the lower quartile, the median, the upper quartile and the maximum —
plus any outliers. It is compact and precise about spread, at the cost of hiding
the shape of the distribution.

## Box plot

<img src="../images/grouped_boxplot.svg" width="500">

```rust
{{#include ../../../examples/grouped_boxplot.rs}}
```

`mark_boxplot` carries its own statistic, so no transform is needed: give it a
categorical `x`, a numeric `y`, and an optional `color` to group the boxes.

## The composable quantile box

When you want the box as a *layer* rather than a finished chart,
`transform_quantile_box` emits just the inter-quartile box and the median as an
ordinary polygon. It uses the same lane layout as `transform_band`, so the box
always sits exactly over its violin.

<img src="../images/violin_box.svg" width="500">

```rust
{{#include ../../../examples/violin_box.rs}}
```

It emits a `box_part` column (`"box"` / `"median"`) so the two polygons can be
styled differently, and copies the `category` / `group` columns back so the box
can be coloured and dodged like the outline it sits on.

## Box on a violin

Because the box is just a layer, it stacks with `.and(…)` over any distribution
view — a plain violin, a dodged violin, or a full raincloud. See
[Violin](violin.md#raincloud).

## See also

- [Violin](violin.md) — the outline form, and the raincloud that combines both.
- [Transforms & Columns](../grammar/transforms.md) — `transform_quantile_box` columns.
