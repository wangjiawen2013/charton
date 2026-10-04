# 1-D Density

A one-dimensional density estimate (a KDE) draws the shape of a numeric column
as a smooth curve — the smooth counterpart of a histogram. There is no density
mark: `transform_density` estimates the curve and `mark_area` draws it.

```text
transform_density   // numeric column  →  (value, density)
  → mark_area       // draw the curve
```

## Density plot

<img src="../images/density.svg" width="500">

```rust
{{#include ../../../examples/density.rs}}
```

Each `color` group gets its own curve. By default the estimator keeps the tails
(`trim = false`), so the curve fades out past the last observation — the
`geom_density` look. A violin is the very same curve stood upright and mirrored;
see [Violin](violin.md).

## Tuning the curve

Estimating the curve (statistics) and drawing it (geometry) are configured
separately:

| Method | Meaning |
|---|---|
| `DensityTransform::with_bandwidth(BandwidthType::Silverman)` | smoothing rule (Scott by default) |
| `.with_kernel(KernelType::Epanechnikov)` | smoothing kernel |
| `.with_trim(true)` | stop each curve at its own data range instead of the shared grid |
| `.with_counts(true)` | scale each curve by its group size (counts, not density) |
| `.with_as(value, density)` | rename the two emitted columns |

`transform_density` **replaces the table**: it reads the numeric column and emits
`(value, density)` plus one column per `groupby` field. See
[Transforms & Columns](../grammar/transforms.md) for the full column contract.

## See also

- [Violin](violin.md) — the upright, mirrored form, and its grouped/split layouts.
- [2-D Density](density_2d.md) — two columns at once.
- [Cumulative Density](cumulative_density.md) — the integral of the curve.
