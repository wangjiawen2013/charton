# 2-D Density

A two-dimensional density estimate describes the **joint** distribution of two
numeric columns. `transform_density_2d` estimates it on a regular grid; the same
grid can then be drawn two ways, both as composite marks:

```text
transform_density_2d                 // (x, y)  →  (x, y, density) grid
  ├─ mark_density_2d                 // filled density heatmap
  └─ mark_contour("density")         // iso-density lines
```

Neither mark hides anything: they expand into `transform_density_2d` plus the
public `mark_rect` / `transform_contour` geometry, so the hand-written recipe is
always available (see [the column contract](#the-column-contract)).

## Density heatmap

<img src="../images/density_heatmap.svg" width="500">

Read the two columns from `x` and `y`; the mark estimates the joint density and
paints one cell per grid node, coloured by the density.

```rust
{{#include ../../../examples/density_heatmap.rs}}
```

Building it by hand is `transform_density_2d` followed by `mark_rect`. `mark_rect`
paints one cell per grid node, so you must set the x and y `bins` to the grid
size or the cells get merged back together — `mark_density_2d` does that for you.

## Density contours

<img src="../images/density_contour.svg" width="500">

The same grid can be passed to `mark_contour("density")`, which turns it into
iso-lines. Colour them by `level` for the familiar `kdeplot` /
`geom_density_2d` picture, or drop the colour for a single-colour contour.

```rust
{{#include ../../../examples/density_contour.rs}}
```

## The column contract

When you build these by hand, both steps **replace the table**, so you need to
know the columns each one emits:

| Call | Reads | Emits |
|---|---|---|
| `transform_density_2d(x, y)` | two numeric columns | `x`, `y`, `density` |
| `transform_contour(x, y, z)` | the grid | `x`, `y`, `path_group`, `level` |

`density` is consumed by the contour step, so it never appears in `encode`.
Rename the outputs with `Density2DTransform::with_as` and
`ContourTransform::with_as` / `with_level_as`. This is the canonical list for
both transforms; the hand-written wire-up is on the
[Contour Plots](contours.md#built-from-the-primitives) page.

## Tuning the estimate

`configure_density_2d` forwards the estimator options:

| Method | Meaning |
|---|---|
| `configure_density_2d(…).with_grid_size(n)` | grid nodes per axis (default 50) |
| `.with_padding(f)` | widen the grid past the data so the tails are visible |
| `.with_bandwidth(BandwidthType::Silverman)` | smoothing rule |
| `.with_color(…)`, `.with_opacity(…)`, `.with_stroke(…)` | the cell visual style |

For a contour, a dashed line is
`configure_contour(\|m\| m.with_dash([6.0, 4.0]))`.

## See also

- [Contour Plots](contours.md) — the general marching-squares contour, for any scalar grid.
- [Heatmaps](heatmaps.md) — `mark_rect` over binned data.
- [Composing Your Own](primitives.md) — recombining the primitives into new pictures.
- [Transforms & Columns](../grammar/transforms.md).
