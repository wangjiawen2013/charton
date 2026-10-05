//! Grouped violins — the two industrial layouts, with `mark_violin`.
//!
//! * **Faceted** (Vega-Lite / Altair style): one violin per panel.
//! * **Dodged** (ggplot2 style): the violins sit side by side inside each
//!   category, and an ordinary `transform_quantile_box` layer draws the inner
//!   box.
//!
//! The penguins `Sex` column has a couple of missing values. `Sex` is the
//! *positional* category here, so those rows are dropped, exactly as ggplot2 and
//! Altair would — no stray `null` violin appears, and the outline and its inner
//! box always agree because both use the shared lane layout.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    // === 1. Faceted: one violin per panel ==================================
    chart!(&penguins)?
        .mark_violin()?
        .configure_violin(|violin| violin.with_opacity(0.7).with_stroke("#7e5109"))
        .encode(alt::y("Body Mass (g)"))?
        .facet(FacetSpec::wrap("Species").with_columns(3))
        .with_title("Body mass by species")
        .with_x_label("Density")
        .with_y_label("Body mass (g)")
        .save("docs/src/images/faceted_violin.svg")?;

    // === 2. Dodged: side by side inside each Sex, with an inner box ========
    let outline = chart!(&penguins)?
        .mark_violin()?
        .configure_violin(|violin| violin.with_color("#d6eaf8").with_stroke("#2c3e50"))
        .encode((
            alt::x("Sex"),
            alt::y("Body Mass (g)"),
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
        .configure_geoshape(|mark| mark.with_fill("white").with_stroke("black"))
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
