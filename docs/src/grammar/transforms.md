# Transforms & the Columns They Produce

A chart pipeline is a sequence of transforms. Each one reads some columns from
the current table and **replaces it** with a new table (or keeps it and adds a
column). To wire the pipeline together you need to know which columns each
transform emits — that is the contract this page records.

The rule to remember:

> **`encode` maps *channels* to *column names*.**
>
> The channels — `x`, `y`, `color`, `path_group`, `size`, `shape`, `text` — are
> the fixed API. The strings you pass them (`alt::x("x")`, `alt::color("level")`)
> are **data column names**, and every transform's outputs are listed below. You
> never have to guess a hidden name; you look it up here (or in the transform's
> own type documentation).

Named columns are what let one transform feed many geometries: the same density
grid can become a heatmap, a contour, or a layer on another chart, so the library
cannot hard-code what the next step wants. You wire it explicitly.

## Three kinds of transform

`transform_*` splits into three groups, and knowing which is which tells you who
calls it.

| Kind | Called by | Examples |
|---|---|---|
| **Public** — a reusable step you call by hand | your code, via `.transform_density(...)` | `transform_density`, `transform_density_2d`, `transform_contour`, `transform_band`, `transform_quantile_box`, `transform_calculate`, `transform_window` |
| **Mark-implied** — a mark's own statistics and prep | the engine, when it builds that mark | `mark_hist` bins and counts; `mark_boxplot` reduces to five numbers; `mark_area`/`mark_bar` stack; `mark_rect` bins; `mark_errorbar` summarises |
| **Composite recipe** — expands a convenience mark into the public steps | the engine, when it builds that mark | `mark_violin` → `transform_density` + `transform_band`; `mark_density` → `transform_density` + area prep; `mark_contour` → `transform_contour`; `mark_density_2d` → `transform_density_2d` + rect prep |

- **Public** transforms are the grammar's building blocks. They take a
  configuration struct (`DensityTransform`, `ContourTransform`, …) and are
  exported in the prelude; the tables below describe what they emit.
- **Mark-implied** transforms are private. They read the mark's configuration and
  the encodings and add the columns that mark needs; you never call them, you
  just choose the mark.
- **Composite recipes** are private too, but they contain no statistics of their
  own — they only assemble public transforms (plus the mark-implied prep) and
  point the encoding at the generated columns. That is why `mark_violin` and a
  hand-written `transform_density` + `transform_band` draw the same picture.

To recombine the pieces yourself, use the **public** transforms: the convenience
marks are just names for common combinations of them. See
[Design Rules: Marks, Tiers & Statistical Atoms](../concepts/design_rules.md).

## Transforms that replace the table

| Transform | Reads | Emits | One row per |
|---|---|---|---|
| `transform_density(field)` | `field` | `value`, `density`; group columns kept | evaluation point × group |
| `transform_density_2d(x, y)` | `x`, `y` | `x`, `y`, `density` | grid node |
| `transform_contour(x, y, z)` | `x`, `y`, `z` | `x`, `y`, `path_group`, `level` | polyline vertex |
| `transform_band(value, width)` (+ `with_center` / `with_group`) | `value`, `width` | `x`, `y`, `path_group`; plus the centre / group columns | polygon vertex |
| `transform_quantile_box(value)` (+ `with_category` / `with_group`) | `value` | `x`, `y`, `path_group`, `box_part`; plus the category / group columns | polygon vertex |

Every emitted name above is only a **default**. Rename it with the transform's
`with_as(...)` method (`transform_contour` also has `with_level_as(...)`), then
use the new names downstream.

## Transforms that add a column

| Transform | Reads | Adds |
|---|---|---|
| `transform_calculate(name, f)` | any column, through `row.val("...")` | `name` |
| `transform_window(...)` | any column | the window output column |

These **keep every existing column**, so the encoding can still refer to the
originals alongside the new one.

## The recurring names

- **`x` / `y`** — Cartesian coordinates. Every point-based geometry uses them.
- **`path_group`** — "these rows form one shape". Any transform that emits
  several independent lines or polygons uses this name, and `mark_path` /
  `mark_polygon` group by it (connecting rows **in order**, without sorting and
  without closing unless the mark closes them).
- **`level`** — the value of an iso-line (contours only).
- **`density`** — an estimated density value (`transform_density`,
  `transform_density_2d`).
- **`value`** — the default name `transform_density` gives its evaluation
  points.

## A worked wire-up

```rust
// 1. Scattered points  →  a density grid: emits x, y, density
.transform_density_2d(Density2DTransform::new("sepal_length", "petal_length"))?
// 2. The grid          →  iso-lines:      emits x, y, path_group, level
.transform_contour(ContourTransform::new("x", "y", "density").with_levels(8))?
.mark_path()?
// 3. Wire the emitted columns to channels
.encode((
    alt::x("x"),
    alt::y("y"),
    alt::path_group("path_group"), // groups the vertices into separate lines
    alt::color("level"),           // colours each line by its level
))?
```

`density` was consumed in step 2, which is why it does not appear in `encode`.

To use your own names instead:

```rust
.transform_density_2d(
    Density2DTransform::new("sepal_length", "petal_length")
        .with_as("gx", "gy", "dens"),
)?
.transform_contour(
    ContourTransform::new("gx", "gy", "dens")
        .with_as("gx", "gy", "line")
        .with_level_as("lvl"),
)?
.encode((alt::x("gx"), alt::y("gy"), alt::path_group("line"), alt::color("lvl")))
```

## Column names are unique

A transform's output columns are its contract with the encoding, so within one
transform every emitted name must be distinct. One case breaks this by accident:
a "replace" transform copies the caller's category / group columns back into the
new table, and those can share a name with a generated column. A tidy table
whose category column is called `x` does exactly that:

```rust
// ✗ `x` is asked to be both the category column and the band's numeric output.
.transform_band(BandTransform::new("value", "density").with_center("x"))
```

Rather than silently overwrite one of the two, the transform returns an error
and asks you to rename the generated column:

```rust
.transform_band(
    BandTransform::new("value", "density")
        .with_center("x")
        .with_as("cx", "cy", "pg"),
)?
.encode((
    alt::x("cx").with_category_labels("x"),
    alt::y("cy"),
    alt::path_group("pg"),
))
```

### What happens to columns you already have

A "replace" transform builds a fresh table, so any column it does not re-emit is
**gone**: a pre-existing `density` or `path_group` column does not clash, it
simply disappears. If you still need it, give the generated column a different
name with `with_as(...)`, or read the original in an earlier layer.
(`transform_density_2d` and `transform_contour` never copy external columns back
in, so they only ever replace.)

## Missing values

A transform that groups rows (a density, a box, a window) records a missing
group value as the reserved `NA` level rather than dropping it. Whether that row
survives is then decided by how the column is used: a positional `center` or `x`
drops it, a non-positional `group` or `color` keeps it (drawn grey). A missing
*value* being summarised is simply skipped. See
[Missing Values & Gaps](../concepts/missing_values.md).

## See also

- [The Layer Pipeline: Data → Stat → Position → Geom](../concepts/grammar_pipeline.md)
- [Contour Plots](../gallery/contours.md) — the density/iso-line recipe in full.
