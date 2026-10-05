# Composing Your Own

The convenience marks cover the charts people ask for by name. This page is for
everything else: how to assemble the public transforms and the Tier-1 geometries
into a picture that no mark provides.

> Read [Design Rules: Marks, Tiers & Statistical Atoms](../concepts/design_rules.md)
> for the tiers and the thin-recipe rule, and
> [Transforms & Columns](../grammar/transforms.md) for the column each transform
> emits.

## The pattern

A hand-written recipe is always three steps:

1. **stat** — a `transform_*` **replaces** the table with derived columns;
2. **geom** — a Tier-1 mark draws those columns;
3. **encode** — map the emitted columns to channels (`x`, `y`, `path_group`,
   `color`).

Every convenience mark is exactly this, and its expansion is written out on its
own page under **"Built from the primitives"**:

| Mark | Hand-written version |
|---|---|
| `mark_density` | [1-D Density → Built from the primitives](density_1d.md#built-from-the-primitives) |
| `mark_violin` | [Violin → Built from the primitives](violin.md#built-from-the-primitives) |
| `mark_density` (`cumulative`) | [Cumulative Density → Built from the primitives](cumulative_density.md#built-from-the-primitives) |
| `mark_contour` | [Contour Plots → Built from the primitives](contours.md#built-from-the-primitives) |

## A new composition: the mirrored density

Recombining means choosing a different *geom* for the same *stat* — or the other
way round. `mark_violin` pairs `transform_density` with `transform_band` (a
mirrored polygon). Pair the same density with `mark_area` and
`stack: "center"` instead and you get a **mirrored density**: each curve is drawn
from `-density / 2` to `+density / 2`. This is the Vega-Lite
`density + area + stack` recipe, and it is the right choice when you want the
untrimmed tail rather than the data-bounded violin.

```rust
{{#include ../../../examples/density_mirror.rs}}
```

## Choosing the geometry

For anything that "connects the dots", the geometry differs by exactly two
flags: *open or closed* and *sorted by x or in row order*.

| Geometry | Order | Grouped by | Closed? | Filled? |
|---|---|---|---|---|
| `mark_line` | sorted by x | `color` | no | no |
| `mark_path` | row order | `path_group` | no | no |
| `mark_polygon` / `mark_geoshape` | row order | `path_group` | yes | yes |

So a contour (`mark_path`, open) and a violin (`mark_polygon`, closed) are the
same geometry one flag apart. See
[Connecting the dots](../grammar/marks.md#connecting-the-dots-line-path-and-polygon).

## When to stop composing

If you keep rebuilding the same picture, that is the signal to add a mark — but
only if it passes the
[promotion checklist](../concepts/design_rules.md). A *split* violin is a
parameter of `mark_violin`; a *raincloud* is a stack of layers, so it stays a
recipe. The rule of thumb: a name that is a geometry/statistic **atom** (violin,
contour) can become a mark; a name that is a **metaphor for a whole picture**
(raincloud, waterfall) is a composition.

## See also

- [Transforms & Columns](../grammar/transforms.md) — the full column contract.
- [Marks & Geometries](../grammar/marks.md) — the Tier-1 vocabulary
  (`mark_point`, `mark_path`, `mark_polygon`, …).
- [Design Rules](../concepts/design_rules.md) — tiers, the thin-recipe rule, and
  when a new mark is justified.
