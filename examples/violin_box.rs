//! Violin with an inner box.
//!
//! The outline is `transform_density` + `transform_band`; the box is
//! `transform_quantile_box`. They share the same lane layout, so the box always
//! sits exactly over its violin.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    let outline = chart!(&penguins)?
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupbys(["Species"])
                .with_trim(true),
        )?
        .transform_band(BandTransform::new("Body Mass (g)", "density").with_center("Species"))?
        .mark_polygon()?
        .configure_geoshape(|m| m.with_fill("#d6eaf8").with_stroke("#2c3e50"))
        .encode((
            alt::x("x").with_category_labels("Species"),
            alt::y("y"),
            alt::path_group("path_group"),
        ))?;

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

    outline
        .and(inner_box)
        .with_x_label("")
        .with_y_label("Body mass (g)")
        .save("docs/src/images/violin_box.svg")?;

    println!("Saved docs/src/images/violin_box.svg");
    Ok(())
}
