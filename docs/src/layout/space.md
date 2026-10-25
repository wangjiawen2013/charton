# Space Manager

In the Grammar of Graphics, defining data layers is only the first step. To create a professional visualization, you must orchestrate the placement of axes, legends, titles, and the plotting area itself. The Space Manager is Charton’s layout engine, responsible for the precise partitioning of the physical canvas.

## The Box Model Philosophy

Charton adopts a nested "Box Model" similar to modern web layout engines. Every chart is composed of concentric rectangular regions, each serving a specific purpose:

1. Plot Panel: The central core where coordinates are resolved and geometric marks are rendered.
2. Bands: Strips of space reserved on the edges of the Plot Panel for the chart title, the legends and the axes. A band only knows how thick it is and which edge it belongs to.
3. Canvas Padding: The external buffer area that keeps the outermost band away from the canvas boundary.

The bands on one edge are stacked from the canvas inwards. The title is always the outermost band on the top edge, so a legend placed at the top ends up *below* the title instead of on top of it. A legend on the left, right or bottom edge sits outside the axes, so it never lands on the axis labels.

Because the title is a band like any other, it only takes space when the chart actually has a title. A chart without one simply leaves that strip to the legend, or to the panel when there is no legend either.

### Aligning the title

The title is aligned inside a frame. By default that frame is the plot panel, so
the title lines up with the data area and with a legend placed at the top.
`with_title_frame(TitleFrame::Figure)` switches the frame to the whole figure
body, which suits a title that belongs to the whole picture rather than to the
data. `with_title_anchor` then picks where inside the frame the title sits:
`Start`, `Middle` (the default) or `End`.

## The Reservation Loop

The size a guide needs is not used directly as a margin. It is fed into a small fixed-point
loop, together with the axes, because the three quantities are mutually dependent.

### The dependency cycle

A legend wraps against the panel, the panel is what remains after the legends *and* the axes have
taken their share, and the axis depth is itself a function of the space left over:

```text
legend size  ->  unclaimed space  ->  axis tick count  ->  axis depth
     ^                                                          |
     |                                                          v
     +----------------  legend budget  <-------------------  panel
```

Concretely, for a `Top` legend:

```text
legend height -> unclaimed_h -> left(y) ticks = AxisView(Left).ticks()
              -> y-axis depth = AxisView(Left).band().depth -> plot_w
              -> legend budget (horizontal main = width) -> wrapped rows -> legend height
```

and for a `Right`/`Left` legend:

```text
legend width -> unclaimed_w -> bottom(x) ticks = AxisView(Bottom).ticks()
             -> x-axis depth = AxisView(Bottom).band().depth -> plot_h
             -> legend budget (vertical main = height) -> wrapped columns -> legend width
```

### The axis pipeline

An axis is resolved in two layers, and both the layout measurement and the drawing go through the
same objects:

| Layer | Owns | Does not know about |
| --- | --- | --- |
| `core::axis::AxisView` | which data scale sits on which physical side (flip), its title, its label rotation, its ticks, and the depth of its band | how tick counts are chosen |
| `core::ticks` | how many ticks fit, given the panel length, the label rotation and the label size | which side the axis is on, its band, its title |

`AxisView` is built from (coordinate, side, panel, explicit ticks, theme). From it the layout reads
`band().depth`, and the renderer reads `ticks()`, `tick_point()` and `band()`. There is exactly one
implementation of the bottom/left mapping and one formula for the band, so a reservation cannot
disagree with what is painted. All rectangular coordinate systems (Cartesian and Geo) draw through
the same renderer; polar keeps its own, because it has a circular frame and no axis band.

### How many ticks an axis gets

A scale snaps a requested count to human-friendly values (`1, 2, 5, 10, ...`).
How many to request is decided by `core::ticks`, which knows the two things the
scale cannot: the panel length and the rendered size of the labels. It is the
only place that decides a tick count, and it runs whenever an `AxisView` is
built, so layout and drawing always see the same list.

By default (no `with_tick_count` on the theme) the rule has three steps:

1. **Length budget.** One label per four tick-font sizes, clamped to 3..10. The
   clamp is what keeps a large canvas from turning into a wall of labels: text
   keeps its physical size, so past a point more labels only add clutter. Ten
   is the upper end used by mainstream plotting libraries (D3 asks for ten;
   Matplotlib's auto locator tops out around nine).
2. **Snap to the scale's ladder.** The budget is passed to the scale, which
   returns its own values (`1/2/5` steps, calendar steps, powers of ten).
   Because it rounds, the list can be slightly denser than requested.
3. **Measure and back off.** The labels that were really produced are projected
   along the axis, rotation included. Neighbours must keep at least one tick
   font size (about two letters) between their boxes, so labels never read as
   one continuous string. A continuous scale is asked for fewer ticks, which
   moves it to its next coarser step; a categorical scale keeps every `n`th
   category. The first list that fits wins.

The count is therefore a *target*, not a promise. Short numbers pack an axis;
long labels (dates, category names) thin it out; and the count does not grow
linearly with the canvas. Fewer labels still span the same domain, so an axis
that drops from nine labels to four stays complete.

Two fallbacks keep the result honest:

- If even the coarsest continuous step, or the sparsest categorical subset,
  does not fit — one label nearly as wide as the axis — only the two ends are
  labelled. Both are real values, and no interval is invented.
- If that is still too sparse for the figure, the labels themselves need
  attention: rotate them (`with_x_tick_label_angle`), shorten them
  (`with_x_label_format` with compact notation or reduced precision), or pin
  ticks.

What overrides the automatic rule:

- `Theme::with_tick_count(n)` pins a target for one figure. It is not clamped to
  3..10 and does not run the fit pass, so the caller owns the density (and any
  overlap on a narrow panel). The scale still snaps to nice values.
- `with_x_ticks([...])` / `with_y_ticks([...])` are a contract: every listed
  value is drawn. The band is measured from those labels, so they are never
  clipped, but they can overlap each other.
- `Theme::with_auto_tick_count()` returns to the default.

### When the cycle can oscillate

The cycle only bites when the axis depth actually depends on the available space. That depends on
the direction the labels are measured in:

- **Bottom x-axis, labels horizontal (the default `x_tick_label_angle = 0`):** the footprint is
  `label_width * |sin| + font_height * |cos| = font_height`, a constant. The tick *count* still
  changes with the panel width, but the *depth* does not — so a left/right legend does **not**
  affect the bottom axis depth in the default theme.
- **Bottom x-axis, rotated labels:** the footprint becomes `label_width * |sin| + ...`, so the
  axis depth depends on which labels are generated, and the cycle is live.
- **Left y-axis, labels horizontal (always):** the footprint is the widest label's *width*, so the
  depth depends on the tick set, and a `Top`/`Bottom` legend can close the cycle even at angle 0.

### Breaking the cycle

`Chart::resolve_scene` starts from a panel that only accounts for the configured percentage
margins, then re-measures until the reservations stop changing:

```rust
const LAYOUT_PASSES: usize = 4;

for _ in 0..LAYOUT_PASSES {
    // The panel as it stands with the bands measured so far.
    let panel_guess = panel_from(content, &reservations, min_panel_size);

    let new_axis_box = calculate_axis_constraints(/* measured against panel_guess */);

    // The legend wraps against the panel it will sit next to.
    let legend_budget = direction.measure(panel_guess.width, panel_guess.height).main;
    let new_plan = pack_guides(specs, position, legend_budget, theme);

    let new_bands = build_bands(
        theme,
        has_title,
        show_legend,
        position,
        legend_thickness(&new_plan, w, h, theme),
        &new_axis_box,
        !is_faceted, // a facet grid keeps its axes inside the grid
    );
    let new_reservations = Reservations::of(&new_bands);

    let settled = new_reservations == reservations && new_axis_box == axis_box;
    reservations = new_reservations;
    axis_box = new_axis_box;
    legend_plan = new_plan;
    bands = new_bands;
    if settled {
        break;
    }
}
```

Three things to note:

- The first pass measures the axes against the whole content area, so its axis depth can be
  provisional. Later passes tighten it.
- `legend_budget` is the *panel's* extent along the packing direction, not the canvas. That is the
  rule that stops a long legend from running past the axis line and covering the tick labels.
- In the common case (axis depth independent of the available space) the loop settles after the
  second pass and exits early. The `settled` check is what makes the extra passes cheap.

### Placing the bands

Once the sizes have settled, every band is turned into a rectangle. The panel is worked out first,
and then the bands are stacked outwards from its edges, innermost first. Because each band is built
from the panel edge outwards, a band can never be drawn on top of the panel, even when the panel is
squeezed all the way down to its smallest allowed size.

The strips are then anchored like this:

| Position | Drawn at | What it follows |
| --- | --- | --- |
| `Top` | `(panel.x, band.y)` | the panel's left edge, starting below the title |
| `Bottom` | `(panel.x, band.y)` | the panel's left edge, starting below the panel |
| `Left` | `(band.x, panel.y)` | the panel's top edge, starting at the canvas's left side |
| `Right` | `(band.x, panel.y)` | the panel's top edge, starting at the panel's right side |

The band was reserved and the band that is drawn are the same rectangle, so the two can never
disagree. There is no special-case shift for the top edge: a top legend simply starts where the
title band ends.

### The floor, and clipping

A legend cannot squeeze the panel out of existence. `legend_thickness` clamps how much room a
legend may reserve:

```rust
min_panel   = max(min_panel_size, canvas * panel_defense_ratio)   // 100px, and 20% of the canvas
max_reserved = max(canvas - min_panel - axis_reserve_buffer, 0)
reserve      = min(plan_size, max_reserved)
```

Only the space *reserved* is clamped; the strip itself is not resized. If the legend is too large
even for the clamped reservation, it overflows, and the renderer clips the drawing to the band
reserved for it. The legend shows its leading entries and the rest is cut off, rather than being
painted over the data. `LegendRenderer` emits a one-line warning when this happens, because a
silently truncated legend is easy to miss.

## Known Limitation: the Layout Loop is Not Guaranteed to Converge

The loop above is a fixed-point iteration on a map that is **piecewise constant**, and such a map
is not guaranteed to have a fixed point. Both the axis depth and the wrapped legend size are
quantised step functions:

- the automatic tick count is a step function of the panel length, and the *set* of labels it
  produces decides the axis depth (the widest label, and for rotated labels the deepest box);
- the legend jumps by a whole column or row every time it wraps.

When the thresholds of those two step functions interleave, the iteration can settle into a
**period-2 cycle** and never stop changing. `LAYOUT_PASSES = 4` is a hard cap, not a convergence
proof, and the loop does not assert that it settled.

### A reproducible case

A clean oscillation appears for a `Top` or `Bottom` legend with a long, wrapping legend and a
continuous `x` axis:

- data: 120 points, `x = i * 123457`, `y = i^3 * 1234`, 40 `"Category N"` groups on color;
- theme: default (automatic tick density), `LegendPosition::Top` or `Bottom`;
- canvas: height 480, width swept from 320 to 1000 (step 2).

With horizontal x labels, **5 of the 341 widths** never converge (widths 730–738); with the labels
rotated 90 degrees, another 5 (widths 844–852). The same chart with `Right` or `Left` legends, and
the same chart with `with_tick_count(20)`, settle on every width of the sweep.

The mechanism is a negative feedback loop:

```text
bigger top legend -> shorter panel -> fewer y ticks / different labels
-> different y-axis depth -> narrower panel -> legend wraps into another row
-> taller legend -> ...
```

### How often it happens

Sweeps over many canvas sizes and legend positions:

| Scenario | Not settled |
| --- | --- |
| Automatic density, `Right`/`Left` legend, labels at 0°, -45° or 90°, width 320–1000 step 2 | 0 / 1,023 |
| Automatic density, `Top`/`Bottom` legend, same data and sweep | 10 / 2,046 (~0.5%, in narrow 5-pixel windows) |
| Same charts with `with_tick_count(20)` | 0 / 1,364 |
| Discrete x with 15/20/25/30 categories, `Top`/`Bottom`/`Right` legends, 86 widths × 83 heights | 0 / 85,656 |

So it is uncommon, but real, and it only shows up when the automatic density is free to change the
tick count as the panel shrinks. Pinning the count with `Theme::with_tick_count` removes one leg of
the cycle and is the practical workaround.

### Impact

- It never crashes and never paints over the data: the clip band still protects the panel, and the
  plan and its reservation are computed together in the same pass, so the two stay self-consistent.
- The visible effect is that the legend wraps for a panel length that is one column/row away from
  the final panel, so the strip can sit one column/row off from where it was measured. Because the
  reserved band and the drawn strip are the same rectangle, the worst that happens is a slightly
  different wrap, never a legend drawn over the data.
- The result depends on the parity of `LAYOUT_PASSES`: with a cap of 4 the loop stops on one phase
  of the cycle, with a cap of 3 it would stop on the other. That is the clearest sign that it is
  not a unique solution.
- Within one canvas size the output is deterministic; the symptom users see is layout *flipping*
  when the canvas size crosses the threshold.
- The loop does not verify settlement at the end, so this happens silently.

### Current status and possible directions

This is a known, accepted limitation at the moment, not an active bug fix. Cheap mitigations that
would not disturb the charts that already settle:

1. detect the cycle (remember the previous two states) and stop on the conservative one — the
   larger reservation;
2. add a `debug_assert!(settled)` (or a one-time release warning) so it stops being silent;
3. pin the oscillating configuration in a regression test that asserts the legend stays outside
   the panel and the layout is stable.

Eliminating the root cause is a structural change and should be planned as its own piece of work:
either decouple the axis depth from the live available space, or make the reservation monotonic so
the iteration must converge, or replace the self-referential loop with a one-directional pipeline
(measure the legend against a fixed budget, then compress the panel once).

## Layout Control Strategies

The Space Manager offers fine-grained control to ensure visual uniformity across complex plots:

### Synchronized Alignment

Even in multi-panel (faceted) visualizations, the Space Manager enforces "Synchronized Layouts." If Panel A has very long tick labels while Panel B has short ones, the Space Manager calculates the maximum required depth across all panels and applies it uniformly, ensuring that the plotting areas remain perfectly aligned.

### Greedy Stacking (Flex-box logic)

When managing multiple legends or colorbars, the Space Manager applies a strategy similar to Flex-box layout:

* Horizontal Stacking: Legends are laid out in a row; if the content exceeds the canvas width, it automatically wraps to a new row.
* Vertical Stacking: When positioned on the sides, legends are stacked in columns, dynamically adjusting the axis spacing to accommodate the total height required by the consolidated legend blocks.

### Aspect Ratio Preservation

While mark positions are data-driven, the Space Manager respects global aspect ratio constraints. If a fixed aspect ratio is requested, the manager calculates the optimal `Panel` size and treats the resulting excess pixels as additional outer padding, ensuring that the visual representation of the data remains undistorted.

## Coordinate-Driven Compensation

The Space Manager is tightly coupled with the Coordinate System:

* Rotation Compensation: When using Polar coordinates (e.g., in a radial plot), the Space Manager automatically detects the "angular sweep." It triggers a rotation compensation logic that recalculates the collision boundary for labels, preventing them from overlapping with the circular plotting area.
* Flip Awareness: When a user invokes `coord_flip()`, the manager automatically swaps the depth-calculation logic for the axes. It recognizes that vertical labels in a standard plot become horizontal labels in a flipped plot, adjusting the padding calculations accordingly.

## Key Takeaways

* Nested Regions: Layout follows an "inside-out" box model progression.
* Two-Phase Backfilling: The algorithm solves the cyclic dependency between label sizing and canvas allocation.
* Synchronized Layouts: The engine ensures that multiple sub-plots maintain perfect alignment regardless of local content differences.
* Adaptive Compensation: Layout strategies are coordinate-aware, automatically preventing overlaps based on whether the chart is Cartesian or Radial.

