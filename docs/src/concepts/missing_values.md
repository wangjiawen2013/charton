# Missing Values & Gaps

Real data has holes. A sensor goes offline, a survey question is skipped, a join
finds no match. How a chart treats those holes matters: dropping too much hides
evidence, keeping too much invents data.

Charton follows the rule used by **ggplot2** and **Vega-Lite / Altair**, guided
by one question:

> Can the mark still be placed on the canvas?

The answer depends on *which channel* the value is missing from.

## The rule

| What is missing | Example | What Charton does |
| --- | --- | --- |
| **Position** — `x` or `y` | a point with no timestamp | **Drop the row.** You cannot place what has no coordinate. |
| **Non-position** — `color`, `shape`, `size` | a point with a known position but no category | **Keep the row**, drawn as the reserved grey `NA` level. |
| **A transform's grouping** | a violin's `group`, a window's `groupby` | **Keep the row** as an extra `NA` group. |

Why the difference? A missing *position* leaves nothing to draw, so dropping it
loses no information that could have been shown. A missing *colour*, on the
other hand, sits on a perfectly good coordinate — dropping it would throw away a
real observation. And because missingness is often itself meaningful, Charton
shows it instead of hiding it.

`None` and `NaN` are treated identically; both are "missing".

## What a missing colour looks like

A row with a missing colour is drawn in neutral grey and, on a categorical
scale, gets an `NA` entry at the end of the legend:

```rust
// a = [1, 2, 3], b = [10, 20, 30], group = ["A", None, "B"]
chart!(a, b, group)?
    .mark_point()?
    .encode((alt::x("a"), alt::y("b"), alt::color("group")))?;
```

All three points are drawn: the middle one is grey, and the legend shows `A`,
`B`, `NA`. The same happens on a continuous colour scale — a missing value is
grey rather than a colour from the gradient.

A layer that simply does not map colour is unaffected: it keeps the mark's own
configured colour. "No colour channel" and "missing colour value" are different
things.

## Gaps in lines and areas

For `mark_line` and `mark_area`, a missing position is not a stray point to
delete — it is where the series stops and starts again. Charton breaks the path
there, so a failed sensor interval reads as a gap in the curve instead of a
straight line drawn across the outage:

```rust
// b = [1.0, 2.0, f64::NAN, 4.0, 5.0] -> two line segments, not one bridge
chart!(a, b)?
    .mark_line()?
    .encode((alt::x("a"), alt::y("b")))?;
```

## Facets

A row whose **facet** field is missing is dropped from the panel layout, matching
ggplot2 and Vega-Lite: a missing panel key does not open a new panel.

## Where this is implemented

- `ColumnVector::unique_values` is the **positional** category list and never
  contains missing values, so positional scales drop them.
- `ColumnVector::labels_with_missing` is the **non-positional** list: it appends
  the reserved `"NA"` label last, and `ColumnVector::label_with_missing` gives
  the matching per-row label.
- The colour mapper paints the reserved slot grey. Every mark resolves colour
  through one shared function, so the rule cannot drift between mark types.
- `transform_density` records a missing grouping as the reserved level, and
  `lane_layout` decides whether to drop it (a positional `category`) or keep it
  (a non-positional `group`).

## See also

- [Scales & Domains](../grammar/scales.md)
- [Marks & Geometries](../grammar/marks.md)
- [Line Charts](../gallery/line_charts.md)
- [Area Charts](../gallery/area_charts.md)
- [Violin](../gallery/violin.md)
