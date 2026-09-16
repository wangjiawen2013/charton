# Guides: Axis & Legends

In a chart, the data marks (points, lines, bars) are the "what," but the Guides are the "how to read it." Guides act as the translation layer between abstract mathematical scales and human-readable visual cues. In Charton, these are primarily categorized into Axes (which interpret spatial mappings) and Legends (which interpret aesthetic mappings).

## The Guide Hierarchy

Following the Grammar of Graphics, Charton treats Axes and Legends as secondary structures that are semantically derived from the primary `Encoding` specification.

* Axes: Provide spatial context. They map the underlying continuous or discrete `Scale` domain to visual ticks and labels along the dimensions of the plotting area.
* Legends: Provide categorical or gradient context. They map aesthetic scales (Color, Shape, Size) back to labeled groups or color ramps.

## Axis Generation: From Math to Ticks

The creation of an axis is a multi-step process triggered during the rendering lifecycle:

1. Tick Calculation: The system queries the `Scale` domain to determine the ideal intervals. For linear data, it uses a "pretty number" algorithm to ensure labels land on clean integers or decimal fractions. For temporal data, it selects sensible units (e.g., hours, days, or months).
2. Physical Positioning: Using the `Coordinate System` (Cartesian or Polar), the axis generator calculates the pixel location for each tick. In Polar systems, this involves converting radial and angular distances into curved labels.
3. Rotation & Collision: If labels are long or dense, the system calculates their physical footprint in pixels. If a conflict occurs, the system dynamically rotates labels (e.g., to 45-degree angles) to avoid overlap, ensuring readability.

Each axis also claims a **depth** — a margin stripped from the plot panel so the axis is not drawn on top of the data:

| Field (`AxisLayoutConstraints`) | Which axis | Measured as |
| --- | --- | --- |
| `bottom` | the bottom x-axis | a **height** reserved below the panel (tick marks + labels + title) |
| `left` | the left y-axis | a **width** reserved left of the panel |

Do not confuse depth with length. The x-axis *length* is the panel width; its *depth* is `bottom`. The y-axis *length* is the panel height; its *depth* is `left`. This distinction matters later, because the legend only ever squeezes the panel along its own direction, while the axis depth is a function of the space left over.

## Legend Consolidation: The Semantic Bridge

A major architectural feature in Charton is the Semantic Merging of legends. In many visualization libraries, color, shape, and size legends are generated separately, leading to cluttered interfaces. Charton optimizes this:

- Field Mapping: The system scans all aesthetic mappings (Color, Shape, Size) and groups them by their data `Field` name.
- Unified Guide Specs: If `Color` and `Shape` both map to the same field (e.g., "Category"), the system merges their definitions into a single `GuideSpec`.
- Rendering Strategy: The legend renderer then determines the `GuideKind` (Legend vs. ColorBar):
    - Discrete Legend: Used for categorical data. It builds a list of visual symbols (e.g., colored squares, specific shapes) that represent the group.
    - ColorBar: Used for continuous gradients. It renders a continuous strip that maps the data domain to a visual color ramp.

## The Legend Pipeline

Legend handling is split into three stages, and each stage owns exactly one decision:

```text
GuideManager::collect_guides()      group aesthetics by field        -> Vec<GuideSpec>
        |
LayoutEngine::pack_guides()         measure + wrap (two levels)      -> LegendLayoutPlan
        |
LegendRenderer::render_legend()     add plan offsets to the origin   -> pixels
```

The renderer is deliberately "dumb": the plan already carries the final position of every block, every entry and every gradient bar, so the renderer only adds `origin + offset`. Re-deriving layout at draw time is exactly what used to let the measured size and the drawn size drift apart.

## Legend Layout Anatomy

A legend is built from two nested levels. The figure below is a real chart -- every box and
marker is drawn at coordinates taken from the layout engine itself, not sketched by hand.

![Legend layout anatomy](../images/legend_layout_anatomy.svg)

- **① `plot panel`** -- the area the data is drawn in. Everything the legend needs is measured
  against this rectangle, not against the whole canvas.
- **② `strip`** -- the resolved legend plan: the bounding box of every block. Its size is what
  gets reserved as margin around the panel.
- **③ ④ `block`** -- one guide. A block contains its own title *and* all of its entries, so this
  is the box that is packed against the other blocks.
- **⑤ `entry`** -- one row of a legend: the symbol cell, the gap after the symbol, and the label
  text. It contains no spacing towards its neighbours; the packer adds the gaps.
- **⑥ `main`** -- the strip's length along the *packing direction*. A legend on the left or right
  edge is packed vertically, so `main` is a height: 216.4px here.
- **⑦ `cross`** -- the thickness across the packing direction (here the block width, 50.6px). It
  never decides whether something fits; it only decides how far the next column is offset when a
  run wraps.
- **⑧** -- `entry.y` starts at 20.2px rather than 0: a block's coordinates begin at its title, and
  the title strip (13.2px) plus its gap (7px) sit above the first entry.
- **⑨** -- the step between two entries: row 18px + `legend_item_v_gap` 3px.
- **⑩** -- `legend_block_gap` (35px). The second block's offset of 115.2px is the first block's
  height (80.2px) plus this gap.
- **⑪ `budget`** -- how long the strip may become: the panel's extent along the packing direction
  (336.6px here). The strip (⑥) has to stay within it, otherwise the legend spills over the axis
  line and covers the tick labels.

### Two levels, one algorithm

Both levels -- the entries inside a block, and the blocks inside the strip -- are packed by the
same routine, so an offset always means the same thing: *the top-left corner inside my parent*. A
block sits at `strip origin + block offset`, an entry at `block origin + entry offset`, and the
renderer only ever adds those numbers together.

The direction is chosen once, from the legend position, and both levels obey it:

| Position | `Direction` | `main` is | `cross` is |
| --- | --- | --- | --- |
| `Top`, `Bottom` | `Horizontal` | width | height |
| `Left`, `Right` | `Vertical` | height | width |

`Direction::measure(w, h)` and `Direction::resolve(main, cross)` are a plain transpose, which is
why the one packing routine can serve both orientations.

### Wrapping and the cursor

The *cursor* tracks where the next box of the current line begins, which is also how much of the
line is used. A box is placed while its far edge -- `cursor + main` -- stays inside the budget;
otherwise the run wraps and the next line is offset across the packing direction. The new line
starts after the *thickest* box of the previous line, never after its last box, so that a thick
box early in a line cannot be overlapped by the next one:

```text
packing direction ->                      a legend with 30 categories, packed vertically
+------------------------------------+
| cat                                |    column 1 holds entries 0..14;
| ● Category 00   ● Category 15      |    the cursor reached 315px, and adding
| ● Category 01   ● Category 16      |    one more entry (18px) would have passed
| ...             ...                |    the budget of 316.4px, so entry 15
| ● Category 14   ● Category 29      |    starts a new column instead
+------------------------------------+
                    ^
   cross offset of column 2 = column width (93.8) + line gap (15) = 108.8
```

Two properties of the packer are worth stating explicitly, because the rest of the layout depends
on them:

- **Only `main` decides whether a box fits.** A box is oversized only if it is longer than the
  whole budget on its own; then it is placed anyway, on a line of its own, and simply overflows.
  Callers that must not overflow have to check `main_total` afterwards.
- **A line is as thick as its `max(cross)`.** Wrapping advances by the line's thickest box, so
  `cross` never affects placement inside a line, only where the next line starts.

### Measuring one entry

A discrete entry is a single box that is fully determined by the theme and the label text:

```text
|<-- cell=18 -->|<-- marker_text_gap -->|<----- label text ----->|
|      ●       |                       |      Category 00       |
|<------------------- entry main ------------------------------>|
```

- `cell` is a fixed 18px square, so every symbol lines up in one column and every label starts at
  the same offset regardless of the glyph.
- `row = max(18, legend_label_size)`. The entry's cross extent is always this row height.
- No spacing is baked into the box. The gaps are passed to the packer separately, and they swap
  with the direction so that lines stay tight while wraps stay readable:

  | Direction | `gap` (within a line) | `line_gap` (between lines) |
  | --- | --- | --- |
  | `Horizontal` | `legend_col_h_gap` | `legend_item_v_gap` |
  | `Vertical` | `legend_item_v_gap` | `legend_col_h_gap` |

The budget handed to the entries is the panel's extent along the packing direction, minus the
title when the title occupies that axis:

```rust
let entry_budget = match direction {
    Vertical   => (main_budget - title_height - title_gap).max(MIN_ENTRY_BUDGET), // 20px floor
    Horizontal => main_budget,
};
```

### Measuring one block

A block is a title strip wrapped around the packed entries. The title is never an entry; it is
added afterwards:

```rust
Vertical   => block main  = title_height + title_gap + packed.main_total
Horizontal => block cross = title_height + title_gap + packed.cross_total
width = width.max(estimate_text_width(title, title_height)) // a wide title widens the block
entries[i].y += title_height + title_gap                    // entries already sit below the title
```

`title_height = legend_label_size * 1.1` (the title is slightly larger than the labels) and
`title_gap = legend_title_gap`. This is why the anatomy figure shows `entry.y = 20.2px`
(13.2 + 7) rather than 0.

Note what the reserved dimension is: for a vertical legend the *height* is the length along the
packing direction and the *width* is the cross dimension. The strip reserves the **cross**
thickness. The legend is not allowed to grow arbitrarily long; it must fit the panel it sits next
to, and when it does not, it wraps and gets thicker. So the space taken from the panel is the
*result of wrapping*, not an arbitrary demand.

### Colour bars

A colour bar has no discrete entries, so it is measured from the budget directly:

| Direction | Length | Thickness |
| --- | --- | --- |
| `Horizontal` | `main_budget.clamp(150, 300)` | 15px |
| `Vertical` | `(main_budget * 0.7).min(200)` | 15px |

Its tick labels sit under a horizontal bar and beside a vertical one, so the block's cross size is
`thickness + padding + font` in the first case and `thickness + legend_marker_text_gap +
max_label_width` in the second. The tick set (`colorbar_ticks`) is shared by measurement and
rendering, so the widest label that was reserved for is the one that gets drawn.

## Space Reservation: the Panel, the Axes and the Legend

The strip size computed above is not used directly as a margin. It is fed into a small fixed-point
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
legend height -> unclaimed_h -> left(y) tick count = floor(unclaimed_h / tick_min_spacing)
              -> y-axis depth (the widest y label) -> plot_w
              -> legend budget (horizontal main = width) -> wrapped rows -> legend height
```

and for a `Right`/`Left` legend:

```text
legend width -> unclaimed_w -> bottom(x) tick count
             -> x-axis depth -> plot_h -> legend budget (vertical main = height)
             -> wrapped columns -> legend width
```

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
    let unclaimed_w = (border_plot_w - legend_box.left - legend_box.right).max(10.0);
    let unclaimed_h = (border_plot_h - legend_box.top - legend_box.bottom).max(10.0);

    let new_axis_box = calculate_axis_constraints(/* measured against unclaimed */);

    let plot_w = (unclaimed_w - new_axis_box.left).max(min_panel_size);
    let plot_h = (unclaimed_h - new_axis_box.bottom).max(min_panel_size);

    let legend_budget = direction.measure(plot_w, plot_h).main;
    let new_plan = pack_guides(specs, position, legend_budget, theme);
    let new_legend_box = legend_constraints_for_plan(new_plan, w, h, legend_margin, theme);

    let settled = new_legend_box == legend_box && new_axis_box == axis_box;
    legend_box = new_legend_box;
    axis_box = new_axis_box;
    legend_plan = new_plan;
    if settled {
        break;
    }
}
```

Three things to note:

- The first pass measures the axes against a panel that does not yet reserve anything for the
  legend, so its axis depth can be provisional. Later passes tighten it.
- `legend_budget` is the *panel's* extent along the packing direction, not the canvas. That is the
  rule that stops a long legend from running past the axis line and covering the tick labels.
- In the common case (axis depth independent of the available space) the loop settles after the
  second pass and exits early. The `settled` check is what makes the extra passes cheap.

### Anchoring the strip

The renderer derives the strip origin from the **final resolved panel**, so the reservations and
the drawing can never disagree. The four cases are:

| Position | `origin` | What moves |
| --- | --- | --- |
| `Right` | `x = panel.x + panel.width + legend_margin` | follows the panel's right edge |
| `Bottom` | `y = panel.y + panel.height + legend_margin` | follows the panel's bottom edge |
| `Left` | `x = max(panel.x - legend_margin - axis_reserve_buffer - plan.width, 10)` | anchored to the canvas's left edge |
| `Top` | `y = max(panel.y - legend_margin - 0.8 * axis_reserve_buffer - plan.height, 10)` | anchored to the canvas's top edge |

The `Left`/`Top` formulas look like they depend on the strip size, but they cancel against the
reservation the panel already made. For a `Left` legend that fits:

```text
panel.x = left_margin + legend_box.left + axis_box.left,  legend_box.left = plan.width + legend_margin
origin.x = panel.x - legend_margin - axis_reserve_buffer - plan.width
         = left_margin + axis_box.left - axis_reserve_buffer     // independent of plan.width
```

So `Left`/`Top` legends sit against the canvas edge and push the panel away, while `Right`/
`Bottom` legends are pinned to the panel's far edge and move *with* it as the panel shrinks. Both
are the same rule in disguise: a legend always hugs the panel edge it belongs to, plus the theme
margin. (`Top` additionally shifts up by `0.8 * axis_reserve_buffer`, which is why the visible gap
above the panel is larger than `legend_margin` alone.)

### The floor, and clipping

A legend cannot squeeze the panel out of existence. `legend_constraints_for_plan` clamps the
reservation:

```rust
min_panel_w = max(min_panel_size, canvas_w * panel_defense_ratio)   // 100px, and 20% of the canvas
max_width   = max(canvas_w - min_panel_w - axis_reserve_buffer, 0)
reserve     = min(plan.width, max_width) + legend_margin
```

Only the space *reserved* is clamped; the strip itself is not resized. If the legend is too large
even for the clamped reservation, it overflows, and `legend_band` clips the drawing to the
half-plane outside the panel on that side. The legend shows its leading entries and the rest is
cut off, rather than being painted over the data. `LegendRenderer` emits a one-line warning when
this happens, because a silently truncated legend is easy to miss.

## Known Limitation: the Layout Loop is Not Guaranteed to Converge

The loop above is a fixed-point iteration on a map that is **piecewise constant**, and such a map
is not guaranteed to have a fixed point. Both the axis depth and the wrapped legend size are
quantised step functions:

- the tick count is `floor(available / tick_min_spacing)`;
- the legend jumps by a whole column or row every time it wraps.

When the thresholds of those two step functions interleave, the iteration can settle into a
**period-2 cycle** and never stop changing. `LAYOUT_PASSES = 4` is a hard cap, not a convergence
proof, and the loop does not assert that it settled.

### A reproducible case

A clean oscillation appears for a `Right` or `Left` legend with rotated x labels and a dense tick
spacing:

- data: 120 points, `x = i * 123457`, `y = i^3 * 1234`, 40 `"Category N"` groups;
- theme: `LegendPosition::Right`, `x_tick_label_angle = 90`, `tick_min_spacing = 20`;
- canvas: height 480, width swept from 320 to 1000 (step 2).

Across that sweep, **59 of 341 widths never converge** (checked with the pass limit raised to 64).
The traced `legend_box.right` alternates forever:

```text
right: 432.2, 323.4, 432.2, 323.4, ...      // G(432.2) = 323.4, G(323.4) = 432.2
```

The mechanism is a negative feedback loop:

```text
bigger legend reserve -> less width -> fewer rotated x ticks -> shallower bottom axis
-> taller panel -> legend fits in fewer columns -> smaller reserve
-> more width -> more ticks -> deeper bottom axis -> shorter panel -> more columns -> bigger reserve
```

### How often it happens

Two sweeps, both over many canvas sizes:

| Scenario | Not settled |
| --- | --- |
| Default theme (`angle = 0`, `tick_min_spacing = 50`), `Right`/`Left`, any category count | 0 / 25272 |
| Default theme, `Top`/`Bottom` with 2/3/5/8 categories | 0 |
| Default theme, `Top`/`Bottom` with 15–30 categories | 34 / 25272 (~0.13%) |
| Adversarial: rotated labels, `tick_min_spacing = 20`, 40–120 categories | 344 / 36828 (~0.9%), up to ~17% within the worst single combination |

So it is uncommon, but real. It needs all three of: an axis depth that is sensitive to available
space, a legend that wraps, and the two quantisation thresholds interleaving on that particular
canvas size. The failure window is narrow — a few pixels of width usually make it disappear.

### Impact

- It never crashes and never paints over the data: the clip band still protects the panel, and the
  plan and its reservation are computed together in the same pass, so the two stay self-consistent.
- The visible effect is that the legend wraps for a panel length that is one column/row away from
  the final panel, so the strip can extend slightly past the panel edge (into the title or the
  axis-label area, since the band is a generous half-plane).
- The result depends on the parity of `LAYOUT_PASSES`: with a cap of 4 the loop stops on one phase
  of the cycle, with a cap of 3 it would stop on the other. That is the clearest sign that it is
  not a unique solution.
- Within one canvas size the output is deterministic; the symptom users see is layout *flipping*
  when the canvas size crosses the threshold.
- The loop does not verify settlement at the end, so this happens silently.

### Current status and possible directions

This is a known, accepted limitation at the moment, not an active bug fix. Cheap mitigations that
would not disturb the 99.87% of charts that already settle:

1. detect the cycle (remember the previous two states) and stop on the conservative one — the
   larger reservation;
2. add a `debug_assert!(settled)` (or a one-time release warning) so it stops being silent;
3. pin the oscillating configuration in a regression test that asserts the legend stays outside
   the panel and the layout is stable.

Eliminating the root cause is a structural change and should be planned as its own piece of work:
either decouple the axis depth from the live available space, or make the reservation monotonic so
the iteration must converge, or replace the self-referential loop with a one-directional pipeline
(measure the legend against a fixed budget, then compress the panel once).

## Layout-Aware Rendering

Guides are not independent objects; they are aware of the chart's total physical constraints.

- **Space Reservation**: The layout engine asks the guides how much room they need and turns that
  into margins around the plot panel. A legend may never be longer than the panel it sits next to;
  that is the rule which keeps a long legend from covering the axis labels.
- **Anchoring**: Guides use a `PanelContext` to position themselves relative to the final resolved
  panel, so the reserved space and the drawn position always agree.
- **Drawing verbatim**: The resolved plan carries the final position of every block, every entry
  and every gradient bar. Renderers draw those numbers as they are instead of re-deriving them,
  which is what guarantees that the drawn legend matches the space reserved for it.

## Why Consolidation Matters

The automated generation of these guides provides three core advantages:

1. Mathematical Integrity: Because guides are generated directly from the resolved global scales, the labels are guaranteed to match the data precisely. You never have to manually update a label when the data changes.
2. Reduced Visual Noise: By merging multiple aesthetics into a single legend block, the chart keeps the viewer's focus on the data, not on redundant interface elements.
3. Automated Layout: Because the layout manager is aware of guide requirements, you don't need to manually configure margins. The chart automatically adjusts to accommodate the font sizes and number of categories present in your specific dataset.

## Key Takeaways

Axes map space; Legends map aesthetics.

- Guides are derived: They are not manually created, but are inferred directly from the Encoding and Scale specifications.
- Semantic Merging: Multiple aesthetics mapping to the same field are consolidated into a single guide to minimize visual clutter.
- Two levels, one packer: entries are packed into blocks, blocks into the strip, by the same flex-wrap routine in a `main`/`cross` frame.
- Layout Awareness: Guides communicate their size requirements to the layout engine, and the panel, the axes and the legends are resolved together by a short fixed-point loop.
- Known limitation: that loop is a fixed-point iteration over quantised values and is not guaranteed to converge; a rare period-2 oscillation can leave the legend layout one column off and flip it as the canvas is resized.