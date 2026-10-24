# Heatmaps

A heatmap is `mark_rect` over two position axes, coloured by a value. The only
question is whether the axes are categorical or continuous.

`mark_rect` is a **per-row cell** mark: it requires `x`, `y` and `color`, is
always axis-aligned, and derives its size from the bin/cell layout. It does not
connect rows and does not use `path_group`. For an arbitrary closed region —
especially one with holes — use `mark_polygon` instead; a rectangle is only a
special case of a polygon, while `mark_rect` is the heatmap/binning contract.

## Categorical heatmap

<img src="../images/heatmap.svg" width="500">

Each `(x, y)` pair is one cell, coloured by its value.

```rust
{{#include ../../../examples/heatmap.rs}}
```

## Continuous heatmap (2-D binning)

<img src="../images/heatmap_rect.svg" width="500">

The same mark with automatic binning on both axes — a "2-D histogram" heatmap.

```rust
{{#include ../../../examples/heatmap_rect.rs}}
```

## See also

- [Contour Plots](contours.md) — the smooth, iso-line alternative to a binned
  heatmap, including [density contours](density_2d.md#density-contours).
