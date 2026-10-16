# Writing Transforms in the Nushell Plugin

Some numbers a chart needs are not in the incoming table: a smooth density
curve, a running total, an inter-quartile box. The plugin's marks draw the table
as it is, so `--transform` computes those columns first and the mark then draws
the result.

```nu
# The mark draws the rows as they are.
open data.csv | charton bar -x region -y revenue

# The transform turns `score` into a curve; the mark draws the curve.
open data.csv | charton area -x value -y density \
    --transform {op: density, field: score, as: [value, density]}
```

This page is the long-form guide. It explains what each key of a transform
record means, lists every transform and its options, and builds the common
recipes (ridgeline, violin-with-box, ECDF, waterfall, raincloud) from the pieces.

Related pages:

- [The Nushell Plugin](nushell.md) — the conceptual overview.
- [Transforms & the Columns They Produce](../grammar/transforms.md) — the
  library-level contract: which Rust method emits which column.
- The plugin
  [README](https://github.com/wangjiawen2013/charton/blob/main/nu_plugin_charton/README.md)
  — the quick reference and the full flag list.

## The shape of a transform

A transform is a Nushell **record**. It has three kinds of keys:

| Key | Meaning |
|---|---|
| `op` | Which operation to run. Required; one of the names in [the catalogue](#the-transform-catalogue). |
| **input keys** | The inputs the operation expects. The key name is fixed by the transform; its **value is one of your column names**. A `density` expects `field`, so `{op: density, field: score}` reads the `score` column; a `density-2d` expects `x` and `y`, so `{op: density-2d, x: a, y: b}` reads columns `a` and `b`. |
| `as` | The names to give the columns the transform writes. Usually a list, e.g. `as: [value, density]`. Omit it to accept the defaults. |

Every other key (`bandwidth`, `trim`, `kind`, ...) tunes that operation. The
catalogue below lists each operation's input keys under **Reads**; the value you
give each key is one of your column names.

The density example, key by key:

```nu
# op:    compute a density
# field: read the `score` column
# as:    name the new columns `value` and `density`
let curve = {op: density, field: score, as: [value, density]}
open data.csv | charton area -x value -y density --transform $curve
```

| Stage | Columns |
|---|---|
| piped in | `score` — one row per observation |
| the transform reads | `score` |
| the transform writes | `value`, `density` — one row per curve point |
| the mark draws | `-x value`, `-y density` |

That one record turns a column of observations into a column of curve points:

```text
before          after (one row per curve point)
score           value   density
1.2             -0.4    0.01
2.0              0.0    0.04
3.1              0.4    0.09
...              ...     ...
```

## Transforms are a pipeline

`--transform` accepts one record, or a **list** run in order. Each step
**replaces** the table, so the next step — and finally the mark's `-x` / `-y` /
`--path-group` — names the columns the previous step wrote (its `as` names),
not the original columns.

A scatter turned into contour lines is the clearest two-step example:

```nu
open data.csv |
charton path -x x -y y --path-group path_group -c level --transform [
    # 1. points → a density grid: writes `x`, `y`, `density`
    {op: density-2d, x: sepal_length, y: petal_length}
    # 2. the grid → iso-lines: writes `x`, `y`, `path_group`, `level`
    {op: contour, x: x, y: y, z: density}
]
# The mark reads the last step's columns.
```

`density` was consumed by step 2, which is why the mark never mentions it. If you
prefer your own names, set `as` on each step and use those names downstream:

```nu
--transform [
    {op: density-2d, x: sepal_length, y: petal_length, as: [gx, gy, dens]}
    {op: contour, x: gx, y: gy, z: dens, as: [gx, gy, line], level-as: lvl}
]
# ...mark -x gx -y gy --path-group line -c lvl
```

## The transform catalogue

The vocabulary is **closed** — these are all the transforms:

| `op` | What it computes | Reads | Writes (`as` default) | Example record |
|---|---|---|---|---|
| `density` | 1-D kernel density: a smooth curve | `field` | `<field>`, `density` | `{op: density, field: score}` |
| `density-2d` | 2-D kernel density on a grid | `x`, `y` | `x`, `y`, `density` | `{op: density-2d, x: a, y: b}` |
| `contour` | Iso-lines of an `x`/`y`/`z` grid | `x`, `y`, `z` | `x`, `y`, `path_group`, `level` | `{op: contour, x: x, y: y, z: z}` |
| `band` | A ribbon of a given width around a value (half a violin, a ridge) | `value`, `width` | `x`, `y`, `path_group` | `{op: band, value: value, width: density, center: species}` |
| `quantile-box` | The inter-quartile box outline | `value` | `x`, `y`, `path_group` | `{op: quantile-box, value: score, category: species}` |
| `window` | A running or ranked statistic: one row in, one row out | `field` | the `kind` name | `{op: window, field: value, kind: rank, group-by: [year]}` |
| `calculate` | Any Nushell expression, one row at a time | any columns | the `as` name (required) | `{op: calculate, as: ratio, expr: {\|row\| $row.a / $row.b}}` |

The `as` names are what the next step (or the mark) sees. The `op` accepts
aliases too: `kde` (`density`), `density2d` (`density-2d`), `box`
(`quantile-box`), and `calc` (`calculate`). All names are matched
case-insensitively with `-` and `_` ignored.

### Options by transform

| `op` | Keys |
|---|---|
| `density` | `bandwidth` (in data units; default Scott's rule), `kernel` (`normal` / `epanechnikov` / `uniform`), `cumulative`, `counts`, `trim`, `group-by` |
| `density-2d` | `grid-size` (default 50), `bandwidth`, `padding` |
| `contour` | `levels` (a count, or a list of values), `level-as` (name of the level column; default `level`) |
| `band` | `center` (the categorical x), `group`, `side` (`both` / `left` / `right`), `scale` (`per-group` / `global` / `raw`), `overlap`, `span`, `band-width`, `split`, `position` (`identity` / `dodge`) |
| `quantile-box` | `category` (the categorical x), `group`, `width`, `box-width`, `span`, `position` (`identity` / `dodge`) |
| `window` | `kind` (`rank` / `dense-rank` / `percent-rank` / `cume-dist` / `cumulative-sum` / `row-number`), `group-by`, `normalize` |
| `calculate` | `expr`: a `{\|row\| ...}` closure, run once per row |

> `band` reads a `width` **column**; to set the ribbon's thickness use the
> `band-width` key instead, or the two are easy to confuse.

### The `window` kinds

`window` keeps the row count and adds one column. The `kind` picks the statistic,
and is also the default `as` name:

| `kind` | Adds |
|---|---|
| `rank` | Competition rank of `field` within the partition |
| `dense-rank` | Rank without gaps |
| `percent-rank` | Rank as a fraction |
| `cume-dist` | The empirical CDF at each row |
| `cumulative-sum` | A running total, in row order |
| `row-number` | The row's position |

Partition with `group-by: [col, ...]`; for example a per-year rank:

```nu
{op: window, field: value, kind: rank, as: rank, group-by: [year]}
```

### `calculate` and `insert`

`calculate` runs a Nushell closure for every row, and the row is a record, so
the closure can read any column an earlier transform produced:

```nu
{op: calculate, as: base, expr: {|row| $row.total - $row.delta}}
```

When a value only depends on the columns already in the table, a plain `insert`
before `charton` is faster, because no per-row closure has to cross the plugin
boundary:

```nu
open data.csv | insert ratio {|r| $r.revenue / $r.cost} | charton point -x ratio -y score
```

Reach for `calculate` when you need a transform's output, or when the value
depends on a column a `window` just added.

## Worked recipes

Every "chart type" below is a composition of the transforms above — nothing is
special-cased in the plugin.

### A density curve

The simplest use: one transform, then an area mark.

```nu
let curve = {op: density, field: score, as: [value, density]}
open assets/data.csv | charton area -x value -y density --transform $curve
```

### ECDF

An empirical CDF is the `cume-dist` window plus a step line:

```nu
let ecdf = {op: window, field: score, kind: cume-dist, as: ecdf, group-by: [group]}
open assets/data.csv
| charton line -x score -y ecdf -c group --interpolation step --transform $ecdf
```

### Ridgeline

A ridge is a density, a one-sided band around each category, and a polygon.
`charton violin` is built from the same pieces — density, a two-sided band and a
polygon — so a ridge is a violin that leans to one side and overlaps its
neighbours.

```nu
open assets/data.csv
| charton polygon -x x -y y --path-group pg -c category --flip \
    --x-expand [0.4, 1.5] --opacity 0.85 --stroke '#2c3e50' --stroke-width 1.0 --transform [
      {op: density, field: score, as: [value, density], group-by: [category]}
      {op: band, value: value, width: density, center: category,
       side: right, scale: per-group, overlap: 2.5, band-width: 1.0, as: [x, y, pg]}
    ]
```

A ridge grows past its lane, so the category axis needs headroom above the top
one. `--x-expand [0.4, 1.5]` pads that axis by 0.4 at the low end and 1.5 at the
high end (in category units), so the tallest ridge is not clipped. The two
numbers are `[low, high]`; a single number instead pads both ends by a fraction
of the range.

### Violin with an inner box

`charton violin` is the density outline alone (Charton's `examples/violin.rs`).
The inner-box form (`examples/violin_box.rs`) is that outline plus a
`quantile-box` layer on the same discrete category axis — a recipe, not a mark:

```nu
let box = {geom: polygon, x: qx, y: qy, path_group: qpg,
           transforms: [{op: quantile-box, value: score, category: category, as: [qx, qy, qpg]}]}
open assets/data.csv | charton violin -x category -y score --layer $box
```

### Waterfall

Floating bars run from the total *before* a step to the total *after* it, with
the opening and closing totals anchored at zero. A `window` computes the running
total, and a `calculate` computes each bar's lower edge:

```nu
open assets/waterfall.csv
| charton bar -x step -y base --y2 total -c kind --mark-width 0.6 --transform [
    {op: window, field: delta, kind: cumulative-sum, as: total}
    {op: calculate, as: base, expr: {|row| if $row.kind == 'Total' { 0 } else { $row.total - $row.delta }}}
  ]
```

### Raincloud

A raincloud is the violin atom, an inner box, and the raw points. The box is a
`quantile-box` transform on its own layer. Because the transform's `category`
labels the shared x axis, the three layers land on the same discrete scale:

```nu
let box = {geom: polygon, x: qx, y: qy, path_group: qpg, color: category,
           transforms: [{op: quantile-box, value: score, category: category, as: [qx, qy, qpg]}]}
open assets/data.csv
| charton violin -x category -y score --layer $box --layer {geom: point, x: category, y: score}
```

## Transforms on a layer

A layer that states its own `transforms` rewrites the table for that layer; a
layer that omits them inherits the primary layer's chain. That inheritance is
why the bump's point overlay only has to say `--layer point`:

```nu
# Bump = a rank window on both layers, drawn as a line with its points.
let rank = {op: window, field: value, kind: rank, as: rank, group-by: [year]}
open assets/bump.csv
| charton line -x year -y rank -c country --transform $rank --layer point
```

The point layer sees the same `rank` column as the line. To read the **original**
rows in a layer instead, give it an empty chain (`transforms: []`) or its own
transform list; either replaces the inherited one.

## See also

- [Transforms & the Columns They Produce](../grammar/transforms.md) — the same
  contract at the Rust level.
- [Design Rules: Marks, Tiers & Statistical Atoms](../concepts/design_rules.md)
  — why recipes are compositions rather than commands.
- [Contour Plots](../gallery/contours.md) — the density → iso-line recipe in
  full.
