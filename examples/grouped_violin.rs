//! Grouped violin plot — the two industrial layouts.
//!
//! Charton follows the usual grammar recipes for this, with **no violin-specific
//! transform**:
//!
//! * **Faceted** (Vega-Lite / Altair style): the ordinary density transform
//!   plus an area mark, stacking the density symmetrically with `"mirror"`.
//!   One violin per panel.
//! * **Dodged** (ggplot2 style): the same density transform, now grouped by
//!   `(Sex, Species)`, followed by the general `transform_band` geometry and a
//!   `Position::dodge`. The inner box is the same composition with
//!   `transform_quantile_box`.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    // === 1. Faceted: density + area + mirror ================================
    chart!(&penguins)?
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupbys(["Species"]),
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

    // === 2. Dodged: density + band + Position, plus a quantile box ==========
    let outline = chart!(&penguins)?
        // One density curve per (Sex, Species) cell.
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupbys(["Sex", "Species"]),
        )?
        // Draw each curve as a symmetric band, dodged inside each Sex.
        .transform_band(
            BandTransform::new("Body Mass (g)", "density")
                .with_center("Sex")
                .with_group("Species")
                .with_position(Position::dodge())
                .with_scale(BandScale::PerGroup),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("#d6eaf8")
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x").with_category_labels("Sex"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("Species"),
        ))?;

    let inner_box = chart!(&penguins)?
        .transform_quantile_box(
            QuantileBoxTransform::new("Body Mass (g)")
                .with_category("Sex")
                .with_group("Species")
                .with_position(Position::dodge()),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("white")
                .with_stroke("black")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x").with_category_labels("Sex"),
            alt::y("y"),
            alt::path_group("path_group"),
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
