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

```rust
use charton::prelude::*;

// Sample z = sin(x) * cos(y) on a regular grid.
let n = 41;
let (mut x, mut y, mut z) = (Vec::new(), Vec::new(), Vec::new());
for i in 0..n {
    for j in 0..n {
        let xv = -3.0 + 6.0 * i as f64 / (n as f64 - 1.0);
        let yv = -3.0 + 6.0 * j as f64 / (n as f64 - 1.0);
        x.push(xv);
        y.push(yv);
        z.push(xv.sin() * yv.cos());
    }
}

chart!(x, y, z)?
    .transform_contour(ContourTransform::new("x", "y", "z").with_levels(10))?
    .mark_path()?
    .configure_path(|m| m.with_stroke_width(1.5))
    .encode((
        alt::x("x"),
        alt::y("y"),
        alt::path_group("path_group"),
        alt::color("level"),
    ))?
    .save("contour.svg")?;
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

```rust
// Single-colour contour: no colour channel, the path's stroke is used.
chart!(x, y, z)?
    .transform_contour(ContourTransform::new("x", "y", "z").with_levels(10))?
    .mark_path()?
    .configure_path(|m| m.with_stroke("black").with_stroke_width(1.0))
    .encode((alt::x("x"), alt::y("y"), alt::path_group("path_group")))?;
```

## Density contours

A density contour is the same picture with a bivariate density as the field.
`transform_density_2d` estimates that density onto a grid, and the same contour
transform draws it — the familiar `kdeplot` / `geom_density_2d` picture:

```rust
chart!(iris)?
    .transform_density_2d(
        Density2DTransform::new("sepal_length", "petal_length").with_grid_size(60),
    )?
    .transform_contour(ContourTransform::new("x", "y", "density").with_levels(8))?
    .mark_path()?
    .encode((alt::x("x"), alt::y("y"),
             alt::path_group("path_group"), alt::color("level")))?
    .save("density_contour.svg")?;
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
