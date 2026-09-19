# Multi-View: Faceting & Concatenation

Faceting splits one chart into a matrix of small multiples, one panel per
category (or per combination of categories). Every panel runs the same grammar
over a different slice of the data, which makes it possible to compare shapes
across groups at a glance.

A faceted chart is not just "several charts next to each other". The panels of
one chart form a single visual object, and that imposes three hard constraints:

1. **All panels must be the same size.** Otherwise the same data value maps to a
   different number of pixels in different panels, and the comparison the facet
   exists to enable becomes misleading.
2. **All panels must be aligned.** Column `c` of every row starts at the same x,
   row `r` of every column starts at the same y. The reader scans the grid, not
   individual panels.
3. **Axis furniture must not be repeated between panels.** A y axis is needed
   once on the left edge, not once per panel; the space it takes is not free.

The facet layout engine is the piece that satisfies all three at once.

## The Facet Grid as Tracks

The layout is expressed as **tracks**: a list of column widths and a list of row
heights, much like an HTML table or a CSS grid. A panel is placed at the
intersection of a *panel column* and a *panel row*; axes and headers occupy
their own dedicated tracks.

```text
  column:  [axis-left?] [   panel   ] spacing [axis-left?] [   panel   ] ...
  row:     [  header  ] [   panel   ] [axis-bottom?] spacing ...
```

Because there is exactly one width for all panel columns and one height for all
panel rows, constraint 1 (equal panels) holds by construction, and constraint 2
(alignment) follows from placing panels at cumulative track offsets.

### Why not split the container into equal cells

The obvious alternative is to divide the container into `rows × cols` equal
cells and inset each panel by a fixed padding for the axis, the way a single
chart might. That is what Charton used to do, and it fails in two ways:

* the axis padding is charged to **every** cell, including cells that draw no
  axis, so it silently becomes part of the gap between neighbouring panels;
* it is reserved **in addition to** the axis space the surrounding layout has
  already measured, so the same axis is paid for twice and dead space appears
  along the outer edges.

The track model removes both failures: a track exists only where an axis is
actually drawn, and the gap between two panels is exactly `facet_spacing`.

### Solving the panel size

Let

* `W`, `H` be the container (the area left after margins, legend and any outer
  chrome),
* `A_x` be the width of one y-axis track, `A_y` the height of one x-axis track,
* `n_x`, `n_y` the number of columns/rows that need such a track,
* `hdr` the header-strip height,
* `s = theme.facet_spacing`.

Every row carries a header track, every column/row that draws an axis carries an
axis track, and adjacent tracks are separated by `s`. What remains is the panel
area, shared equally:

```text
                         W - A_x * n_x - (cols - 1) * s
    panel_width   =  ---------------------------------------
                                      cols

                         H - A_y * n_y - rows * hdr - (rows - 1) * s
    panel_height  =  ----------------------------------------------------
                                          rows
```

The two axes are deliberately asymmetric: a header sits **above** each row and
is counted once per row, while a gap is only counted **between** tracks.

### Placing the tracks

Once the panel size is known, each track position is a cumulative sum. For
column `c`, the x of its panel is the container origin plus

* every y-axis track up to and including this column, plus
* the panels and gaps of all the columns before it.

For row `r`, the y of its header is the container origin plus

* every x-axis track of the rows **above** it, plus
* the headers, panels and gaps of all the rows above it.

`FacetGridGeometry` owns exactly these two sums; `panel_rect` and `header_rect`
are the only ways to read them, so a renderer can never disagree with the
measurement about where a panel goes.

### Worked example

`examples/facet_grid.rs` renders a `Fixed` 2×2 grid at 800×500 with a legend on
the right. The measured tracks are:

```text
A_x = 58.3      (y axis: ticks + rotated title)
A_y = 57.0      (x axis: ticks + title)
hdr = 26.5      facet_label_size * 1.5 + facet_strip_padding * 2
s   = 10.0      facet_spacing
W   = 728.4     canvas minus right margin and legend
H   = 500.0
```

`Fixed` puts the y axis on the first column only and the x axis on the last row
only, so `n_x = 1` and `n_y = 1`:

```text
panel_width  = (728.4 - 58.3 - 1*10) / 2 = 330.05
panel_height = (500.0 - 57.0 - 2*26.5 - 1*10) / 2 = 190.0
```

which lays out as:

```text
            x=0        x=58.3                    x=398.35              x=728.4
             │            │                          │                    │
  y=0     ───┼────────────┼──────────────────────────┼────────────────────┤
             │            │  header 0,0              │  header 0,1        │ 26.5
  y=26.5  ───┼────────────┼──────────────────────────┼────────────────────┤
             │            │                          │                    │
             │  y axis    │      panel 0,0           │ 10 │  panel 0,1    │ 190
             │  track     │                          │    │               │
  y=216.5 ───┼────────────┼──────────────────────────┼────────────────────┤
             │            │  header 1,0              │  header 1,1        │ 26.5
  y=253   ───┼────────────┼──────────────────────────┼────────────────────┤
             │            │                          │                    │
             │            │      panel 1,0           │    │  panel 1,1    │ 190
             │            │                          │                    │
  y=443   ───┼────────────┼──────────────────────────┴────────────────────┤
             │            │            x axis track (spans both columns) │ 57
  y=500   ───┴────────────┴───────────────────────────────────────────────┘
```

The only space between `panel 0,0` and `panel 0,1` is the 10 px gap, and the
y-axis track appears once, at the far left.

## Which Tracks Get an Axis

`FacetStrategy` decides, per cell, whether that cell draws its own x and y axes.
The grid turns those decisions into tracks: a column gets a y-axis track if
**any** of its cells draws a y axis, and a row gets an x-axis track if **any** of
its cells draws one.

| Strategy | x-axis shown on             | y-axis shown on          | Tracks reserved           |
| -------- | --------------------------- | ------------------------ | ------------------------- |
| `Fixed`  | bottom-most cell of column  | left-most cell of row    | first column, last row    |
| `Free`   | every cell                  | every cell               | every column and row      |
| `FreeX`  | every cell                  | left-most cell of row    | every column, last row    |
| `FreeY`  | bottom-most cell of column  | every cell               | first column, every row   |

For wrap layouts the "bottom-most cell of a column" can be in the second-to-last
row, because the last row may be incomplete. `FacetStrategy::axis_visibility`
handles that with an `is_last_in_column` flag; the grid simply marks the whole
row as needing an x-axis track, and the equal panel height keeps every panel
aligned even though only one column draws the axis.

## Where the Axis Extents Come From

The track sizes are not hard-coded. A fixed padding cannot know how wide a
tick label such as `1.0000E7` will be, so it either clips long labels or wastes
space on short ones. Instead the surrounding layout measures the axes once with
the same code a single chart uses (`LayoutEngine::calculate_axis_constraints`)
and passes the result down as [`FacetMetrics`]:

```text
FacetMetrics { axis_left, axis_bottom }
```

`FacetMetrics` is deliberately only an input to `compute_panels`: the facet
implementation decides where the tracks go, and the measurement only says how
big they are. This keeps the "what does an axis need" question in one place and
the "where does an axis go" question in another.

## Axis Reservation for Faceted Charts

A single chart reserves its axis space on the outer border of the panel: the
`Space Manager` shrinks the panel from the left and bottom, and the axes live in
the gap. A faceted chart must **not** do that, because the axes do not live on
the outer border — they live inside the facet grid, on specific columns and
rows. Reserving them globally *and* per-cell is what produced the dead band at
the left and bottom edges of a faceted chart.

So for a faceted chart:

* the global axis reservation is skipped, and the full content rectangle is
  handed to the facet layout;
* the measured extents travel to the grid as `FacetMetrics` and become tracks
  there.

The result is that margins behave uniformly across single and faceted charts:
`with_left_margin(0.0)` really does push a faceted grid to the canvas edge,
exactly as it does for a single panel.

## A Note on Concatenation

This chapter covers the facet grid, where panels share one layout and one set of
axis tracks. *Concatenation* -- placing whole, independent charts side by side
without a shared grid -- is a separate, higher-level composition and is not part
of the layout engine yet. Nothing here should be read as describing it.

## Summary

* Facet panels are laid out on **tracks**, not equal cells: axis space is a
  track of its own and appears only where an axis is drawn.
* All panels are equal by construction, and every panel starts on a shared
  column/row line.
* The gap between neighbouring panels is exactly `theme.facet_spacing`.
* Track sizes come from the measured `FacetMetrics`, never from a fixed padding.
* A faceted chart leaves the outer axis reservation to the grid, so margins keep
  their meaning.
