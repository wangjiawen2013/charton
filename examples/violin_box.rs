//! Violin with an inner box.
//!
//! The outline is `mark_violin`; the box is `transform_quantile_box`. They share
//! the same lane layout, so the box always sits exactly over its violin.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    let outline = chart!(&penguins)?
        .mark_violin()?
        .configure_violin(|mark| {
            mark.with_color("#d6eaf8")
                .with_opacity(1.0)
                .with_stroke("#2c3e50")
                .with_stroke_width(0.5)
        })
        .encode((alt::x("Species"), alt::y("Body Mass (g)")))?;

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
