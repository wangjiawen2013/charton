# Contour Plots

A contour plot draws the lines (or bands) where a scalar field equals a set of
levels. It is a **composition**, not a dedicated chart type:

```text
contour = iso-line stat (marching squares)  +  open-path geometry
```

Nothing here is contour-specific. `transform_contour` works on any regular grid
of `x`, `y` and `z` — a bivariate density, an elevation map, a pressure field —
and `mark_path` draws any ordered polyline.

## Iso-lines of a grid

<img src="../images/contour.svg" width="500">

```rust
{{#include ../../../examples/contour.rs}}
```

The transform writes four columns: `x`, `y`, `path_group` (one per extracted
polyline) and `level`. `mark_path` connects each `path_group` in row order and
strokes it with the colour mapped from `level`.

## How the pieces fit

| Piece | What it does | Reused by |
|---|---|---|
| `transform_density_2d` | bivariate kernel density over a grid | density contours, density heatmaps |
| `transform_contour` | marching squares over a regular grid → iso-lines | contours of any scalar field |
| `mark_path` | connects a `path_group` in row order, open, stroked | contour lines, custom curves, parallel coordinates |
| `mark_polygon` | the closed, filled form of the same geometry | violin outlines, maps, filled ribbons |

Why `mark_path` and not `mark_polygon`? A contour is a **line**: some contours
are open (they end on the plot edge), others are loops, but in both cases we
want the stroke, not a filled disc. `mark_polygon` would close every open
contour with a spurious edge and fill every loop. See [Connecting the
dots](../grammar/marks.md#connecting-the-dots-line-path-and-polygon).

`ContourTransform` accepts either a number of evenly spaced levels
(`with_levels(n)`) or explicit ones (`with_levels_values(vec![...])`).

Colouring the lines by `level` is optional. Drop the colour encoding and the
path is drawn with the mark's own stroke instead — a single-colour contour:

<img src="../images/contour_single.svg" width="500">

```rust
{{#include ../../../examples/contour_single.rs}}
```

## Density contours

A density contour is the same picture with a bivariate density as the field.
`transform_density_2d` estimates that density onto a grid, and the same contour
transform draws it — the familiar `kdeplot` / `geom_density_2d` picture:

<img src="../images/density_contour.svg" width="500">

```rust
{{#include ../../../examples/density_contour.rs}}
```

So a density contour is three general pieces in a row:

```text
scatter  →  transform_density_2d  →  transform_contour  →  mark_path
```

## See also

* [Statistical Distributions](statistics.md) — violin and box plots.
* [The Layer Pipeline](../concepts/grammar_pipeline.md) — the stat/position/geom
  model behind all of this.
* `examples/contour.rs`.
