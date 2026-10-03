# Statistical Distributions

This chapter shows how to summarise a numeric column. Charton builds these plots
from ordinary parts — see [The Layer Pipeline](../concepts/grammar_pipeline.md)
for the underlying model (data → stat → position → geom).

## Violin plot

A violin shows the **density** of a numeric column as a symmetric outline. There
are two idiomatic ways to build one, matching the two industry recipes.

### Reuse the density transform (Vega-Lite / Altair recipe)

For a single or faceted violin, no violin-specific transform is needed. Estimate
the density, draw it as an area, and ask the stack to **mirror** the values
symmetrically around zero:

```rust
use charton::prelude::*;

let iris = load_dataset("iris")?;

chart!(&iris)?
    .transform_density(
        DensityTransform::new("sepal_length")
            .with_as("sepal_length", "density")
            .with_groupbys(["species"]),
    )?
    .mark_area()?
    .configure_area(|a| a.with_opacity(0.7).with_stroke("black"))
    .encode((
        alt::x("sepal_length"),
        alt::y("density").with_stack("mirror"), // symmetric band
        alt::color("species"),
    ))?
    .facet(FacetSpec::wrap("species"))
    .coord_flip()
    .save("violin.svg")?;
```

`"mirror"` is Charton's name for Vega-Lite's `stack: "center"` applied *per
series*: each density curve is drawn from `-density / 2` to `+density / 2`. With
one curve per panel (thanks to the facet) the result is a violin. The single
violin is the same picture without the facet.

### Dodged and split violins (ggplot2 `stat_ydensity` recipe)

A **dodged** violin — several violins side by side inside one category — needs
the *category* on x and a two-field `(category, group)` grouping. That is exactly
what the density transform's multi-field `with_groupbys([...])` provides. The
curve then becomes a polygon through the general band geometry, with a
`Position` deciding the lanes:

```rust
let penguins = load_dataset("penguins")?;

chart!(&penguins)?
    .transform_density(
        DensityTransform::new("Body Mass (g)")
            .with_as("Body Mass (g)", "density")
            .with_groupbys(["Sex", "Species"]), // (category, group)
    )?
    .transform_band(
        BandTransform::new("Body Mass (g)", "density")
            .with_center("Sex")   // x position
            .with_group("Species") // one band per species
            .with_position(Position::dodge())
            .with_scale(BandScale::PerGroup),
    )?
    .mark_polygon()?
    .configure_geoshape(|mark| mark.with_fill("#d6eaf8").with_stroke("#2c3e50"))
    .encode((
        // Numeric positions, but the axis shows the "Sex" categories.
        alt::x("x").with_category_labels("Sex"),
        alt::y("y"),
        alt::path_group("path_group"),
        alt::color("Species"),
    ))?
    .save("grouped_violin.svg")?;
```

Both recipes share the same `stats::kde` core, so no statistics are duplicated.
Add `.with_split(true)` to `BandTransform` and the two groups become the two
halves of one violin.

### Tuning the shape

Density (statistics) and band (geometry/placement) are configured separately:

| Method | Meaning |
|---|---|
| `BandTransform::with_scale(BandScale::PerGroup)` | every band has the same maximum width (default) |
| `.with_scale(BandScale::Global)` | bands share one scale, so a denser group looks wider |
| `.with_scale(BandScale::Raw)` | use the width column as it stands |
| `BandTransform::with_width(0.5)` | maximum width of a single band (matches the box plot) |
| `.with_span(0.7)` | total width of a category's group (matches the box plot and point marks) |
| `BandTransform::with_split(true)` | draw two groups as one split violin |
| `DensityTransform::with_bandwidth(BandwidthType::Silverman)` | smoothing rule |
| `.with_kernel(KernelType::Epanechnikov)` | smoothing kernel |

## Inner box and median

`transform_quantile_box` produces the inner inter-quartile box and the median
line as a second polygon layer. It uses the same `Position` lane layout as
`transform_band`, so the box always sits exactly over its violin:

```rust
let outline = chart!(&penguins)?
    .transform_density(
        DensityTransform::new("Body Mass (g)")
            .with_as("Body Mass (g)", "density")
            .with_groupbys(["Species"]),
    )?
    .transform_band(BandTransform::new("Body Mass (g)", "density").with_center("Species"))?
    .mark_polygon()?
    .encode((alt::x("x"), alt::y("y"), alt::path_group("path_group")))?
    .configure_geoshape(|m| m.with_fill("#d6eaf8"))?;

let box = chart!(&penguins)?
    .transform_quantile_box(QuantileBoxTransform::new("Body Mass (g)").with_category("Species"))?
    .mark_polygon()?
    .configure_geoshape(|m| m.with_fill("white").with_stroke("black"))?
    .encode((alt::x("x"), alt::y("y"), alt::path_group("path_group")))?;

outline.and(box).save("violin_with_box.svg")?;
```

The transform emits a `box_part` column (`"box"` / `"median"`) so the two
polygons can be styled differently if you want.

## Box plot

The box plot is the five-number summary drawn directly. It carries its own
statistics, so no transform is needed:

```rust
chart!(&penguins)?
    .mark_boxplot()?
    .encode((
        alt::x("Sex"),
        alt::y("Body Mass (g)"),
        alt::color("Species"),
    ))?
    .save("grouped_boxplot.svg")?;
```

## Choosing between violin, box and swarm

* **Box plot** — compact, precise about quartiles, hides the shape.
* **Violin** — shows the shape; good when the distribution is interesting.
* **Beeswarm / strip** — shows every observation; good for small samples.

They combine well: a violin outline with a box, a median line or a swarm layered
on top (a *raincloud*) uses `.and(…)` to stack the layers.
