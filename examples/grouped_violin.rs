//! Grouped violin plot — the two industrial layouts.
//!
//! Charton follows the usual grammar recipes for this:
//!
//! * **Faceted** (Vega-Lite / Altair style): reuse the ordinary density transform
//!   and area mark, stacking the density symmetrically with `"mirror"`. One
//!   violin per panel. No violin-specific transform is needed.
//! * **Dodged** (ggplot2 style): `transform_violin` is the `stat_ydensity`
//!   equivalent. It is only necessary here because dodging needs the *category*
//!   on x and a two-field `(category, group)` grouping, which the density
//!   transform cannot express. Its inner box is a second polygon layer.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    // === 1. Faceted: existing transforms only (density + area + mirror) =====
    chart!(&penguins)?
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupby("Species"),
        )?
        .mark_area()?
        .configure_area(|a| a.with_opacity(0.7).with_stroke("#7e5109"))
        .encode((
            alt::x("Body Mass (g)"),
            alt::y("density").with_stack("mirror"),
            alt::color("Species"),
        ))?
        .facet(FacetSpec::wrap("Species").with_columns(3))
        .coord_flip()
        .with_title("Body mass by species")
        .with_x_label("Density")
        .with_y_label("Body mass (g)")
        .save("docs/src/images/faceted_violin.svg")?;

    // === 2. Dodged: stat_ydensity-style transform + inner box ===============
    let params = ViolinTransform::new("Body Mass (g)")
        .with_category("Sex")
        .with_group("Species")
        .with_position(Position::dodge())
        .with_scale(ViolinScale::Width);

    let outline = chart!(&penguins)?
        .transform_violin(params.clone())?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("#d6eaf8")
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x").with_category_labels("Sex"),
            alt::y("y"),
            alt::path_group("violin_id"),
            alt::color("Species"),
        ))?;

    let inner_box = chart!(&penguins)?
        .transform_violin_box(params)?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("white")
                .with_stroke("black")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x").with_category_labels("Sex"),
            alt::y("y"),
            alt::path_group("violin_id"),
        ))?;

    outline
        .and(inner_box)
        .with_title("Body mass by species and sex")
        .with_x_label("")
        .with_y_label("Body mass (g)")
        .save("docs/src/images/grouped_violin.svg")?;

    println!("Saved docs/src/images/faceted_violin.svg and grouped_violin.svg");
    Ok(())
}
