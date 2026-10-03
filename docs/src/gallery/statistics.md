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
            .with_groupby("species"),
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

### `transform_violin` (ggplot2 `stat_ydensity` recipe)

A **dodged** violin — several violins side by side inside one category — cannot
use the recipe above: the density transform puts the *measured value* on x and
can only group by a single field, while dodging needs the *category* on x and a
two-field `(category, group)` grouping. `transform_violin` fills exactly that
gap. It mirrors the curve and applies a `Position`, then emits a polygon:

```rust
let penguins = load_dataset("penguins")?;

chart!(&penguins)?
    .transform_violin(
        ViolinTransform::new("Body Mass (g)")
            .with_category("Sex")   // x position
            .with_group("Species")  // one curve per species
            .with_position(Position::dodge())
            .with_scale(ViolinScale::Width),
    )?
    .mark_polygon()?
    .configure_geoshape(|mark| mark.with_fill("#d6eaf8").with_stroke("#2c3e50"))
    .encode((
        // Numeric positions, but the axis shows the "Sex" categories.
        alt::x("x").with_category_labels("Sex"),
        alt::y("y"),
        alt::path_group("violin_id"),
        alt::color("Species"),
    ))?
    .save("grouped_violin.svg")?;
```

Both recipes reuse the same `stats::kde` core, so they produce the same
statistics.

### Tuning the shape

| Method | Meaning |
|---|---|
| `.with_scale(ViolinScale::Width)` | every violin has the same maximum width |
| `.with_scale(ViolinScale::Area)` | every violin encloses the same area (default) |
| `.with_scale(ViolinScale::Count)` | wider violins for larger samples |
| `.with_width(0.5)` | maximum width of a single violin (matches the box plot) |
| `.with_span(0.7)` | total width of a category's group (matches the box plot and point marks) |
| `.with_bandwidth(BandwidthType::Silverman)` | smoothing rule |
| `.with_kernel(KernelType::Epanechnikov)` | smoothing kernel |
| `.with_steps(200)` | points per side; larger is smoother |
| `.with_trim(false)` | extend the tails a little |

## Inner box and median

`transform_violin_box` produces the inner inter-quartile box and the median line
as a second polygon layer. It runs the same statistics and lane layout as
`transform_violin`, so the box always sits exactly over its violin:

```rust
let outline = chart!(&penguins)?
    .transform_violin(params.clone())?
    .mark_polygon()?
    .encode((alt::x("x"), alt::y("y"), alt::path_group("violin_id")))?
    .configure_geoshape(|m| m.with_fill("#d6eaf8"))?;

let box = chart!(&penguins)?
    .transform_violin_box(params)?
    .mark_polygon()?
    .configure_geoshape(|m| m.with_fill("white").with_stroke("black"))?
    .encode((alt::x("x"), alt::y("y"), alt::path_group("violin_id")))?;

outline.and(box).save("violin_with_box.svg")?;
```

The transform also writes `y_q1`, `y_median` and `y_q3` columns, so other
overlays can use them directly.

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
