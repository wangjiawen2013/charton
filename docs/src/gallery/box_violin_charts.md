# Box & Violin Combinations

The violin chapter built a single violin from a density stat, a position and a
polygon. Because those parts are independent, they can be rearranged and stacked
into the richer pictures researchers commonly need. Nothing below adds a new
mark — every picture is a stack of ordinary layers.

## Raincloud: violin + box + points

A *raincloud* shows the same distribution three ways: a density outline, a box
plot for the quartiles, and the raw observations. Each view is a layer, and all
of them share one scale so they line up automatically.

```rust
use charton::prelude::*;

let penguins = load_dataset("penguins")?;

// The statistics are shared between the outline and the box.
let params = ViolinTransform::new("Body Mass (g)")
    .with_category("Species")
    .with_scale(ViolinScale::Width);

// 1. The cloud: the density outline.
let violin = chart!(&penguins)?
    .transform_violin(params.clone())?
    .mark_polygon()?
    .configure_geoshape(|m| m.with_fill("#d6eaf8").with_stroke("#2c3e50"))
    .encode((
        alt::x("x").with_category_labels("Species"),
        alt::y("y"),
        alt::path_group("violin_id"),
    ))?;

// 2. The box: quartiles + median, from the same statistics.
let inner_box = chart!(&penguins)?
    .transform_violin_box(params)?
    .mark_polygon()?
    .configure_geoshape(|m| m.with_fill("white").with_stroke("black"))
    .encode((
        alt::x("x").with_category_labels("Species"),
        alt::y("y"),
        alt::path_group("violin_id"),
    ))?;

// 3. The rain: every observation, jittered inside the violin.
let rain = chart!(&penguins)?
    .mark_point()?
    .configure_point(|p| p.with_layout("jitter").with_size(2.5).with_opacity(0.55))
    .encode((alt::x("Species"), alt::y("Body Mass (g)")))?;

violin.and(inner_box).and(rain).save("raincloud.svg")?;
```

## Split violin

A split violin contrasts two groups inside one outline: the first group grows to
the right of the category centre, the second to the left. Ask the transform for
`with_split(true)` and colour the result by group — the two halves then read as a
single violin split down the middle.

```rust
chart!(&penguins)?
    .transform_violin(
        ViolinTransform::new("Body Mass (g)")
            .with_category("Species")
            .with_group("Sex")
            .with_split(true)
            .with_scale(ViolinScale::Width),
    )?
    .mark_polygon()?
    .configure_geoshape(|m| m.with_fill("#95a5a6").with_stroke("#2c3e50"))
    .encode((
        alt::x("x").with_category_labels("Species"),
        alt::y("y"),
        alt::path_group("violin_id"),
        alt::color("Sex"),
    ))?
    .save("split_violin.svg")?;
```

## Overlaying a box on a normally-dodged violin

`transform_violin` also writes `y_q1`, `y_median` and `y_q3` columns, so any
overlay can read them directly. For a dodged (side-by-side) violin, layer
`transform_violin_box` on top exactly as in the raincloud, but keep the
`Position::Dodge` on both transforms so the boxes follow their violins.

## Why this is better than a dedicated "violin mark"

Every combination above — raincloud, split, overlay, grouped, faceted — would
need bespoke branching inside a single `MarkViolin`. Building from a density
stat plus a polygon instead means:

* the polygon renderer is shared with maps and custom shapes;
* new combinations are new layers, not new renderers;
* every rendering backend (SVG, PNG, PDF, GPU) already knows how to draw the
  parts.

## See also

* [Statistical Distributions](statistics.md) — the basic violin and box plots.
* [The Layer Pipeline](../concepts/grammar_pipeline.md) — the stat/position/geom
  model behind these examples.
* `examples/raincloud.rs`, `examples/split_violin.rs`, `examples/violin.rs`,
  `examples/grouped_violin.rs`.
