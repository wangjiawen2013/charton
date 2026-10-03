//! Split violin — two groups as the left and right halves of one violin.
//!
//! A split violin keeps one centre line per category and grows the first group
//! to the right and the second to the left. Each half is an ordinary polygon, so
//! colouring by group turns the two halves into a single violin split down the
//! middle. Nothing new is needed at the rendering level.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    chart!(&penguins)?
        .transform_violin(
            ViolinTransform::new("Body Mass (g)")
                .with_category("Species")
                .with_group("Sex")
                .with_split(true)
                .with_scale(ViolinScale::Width),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("#95a5a6")
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x").with_category_labels("Species"),
            alt::y("y"),
            alt::path_group("violin_id"),
            alt::color("Sex"),
        ))?
        .with_title("Body mass by species and sex (split)")
        .with_x_label("")
        .with_y_label("Body mass (g)")
        .save("docs/src/images/split_violin.svg")?;

    println!("Saved docs/src/images/split_violin.svg");
    Ok(())
}
