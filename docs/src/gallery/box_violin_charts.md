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

// 1. The cloud: the density outline (stat + general band geometry).
let violin = chart!(&penguins)?
    .transform_density(
        DensityTransform::new("Body Mass (g)")
            .with_as("Body Mass (g)", "density")
            .with_groupbys(["Species"]),
    )?
    .transform_band(
        BandTransform::new("Body Mass (g)", "density").with_center("Species"),
    )?
    .mark_polygon()?
    .configure_geoshape(|m| m.with_fill("#d6eaf8").with_stroke("#2c3e50"))
    .encode((
        alt::x("x").with_category_labels("Species"),
        alt::y("y"),
        alt::path_group("path_group"),
    ))?;

// 2. The box: quartiles + median, from the same observations.
let inner_box = chart!(&penguins)?
    .transform_quantile_box(
        QuantileBoxTransform::new("Body Mass (g)").with_category("Species"),
    )?
    .mark_polygon()?
    .configure_geoshape(|m| m.with_fill("white").with_stroke("black"))
    .encode((
        alt::x("x").with_category_labels("Species"),
        alt::y("y"),
        alt::path_group("path_group"),
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
the right of the category centre, the second to the left. Density groups by the
two fields, and the band geometry is asked for `with_split(true)`; colour the
result by group and the two halves read as a single violin split down the
middle.

```rust
chart!(&penguins)?
    .transform_density(
        DensityTransform::new("Body Mass (g)")
            .with_as("Body Mass (g)", "density")
            .with_groupbys(["Species", "Sex"]),
    )?
    .transform_band(
        BandTransform::new("Body Mass (g)", "density")
            .with_center("Species")
            .with_group("Sex")
            .with_split(true),
    )?
    .mark_polygon()?
    .configure_geoshape(|m| m.with_fill("#95a5a6").with_stroke("#2c3e50"))
    .encode((
        alt::x("x").with_category_labels("Species"),
        alt::y("y"),
        alt::path_group("path_group"),
        alt::color("Sex"),
    ))?
    .save("split_violin.svg")?;
```

## Overlaying a box on a normally-dodged violin

`transform_quantile_box` computes the quartiles per `(category, group)` cell, so
any overlay can read them directly. For a dodged (side-by-side) violin, layer it
on top exactly as in the raincloud, but keep the same `Position::Dodge` on the
band and on the box so the boxes follow their violins.

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
