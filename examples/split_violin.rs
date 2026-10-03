//! Split violin — two groups as the left and right halves of one violin.
//!
//! A split violin keeps one centre line per category and grows the first group
//! to the right and the second to the left. It is the ordinary density transform
//! followed by the general band geometry in `split` mode; each half is an
//! ordinary polygon, so colouring by group turns the two halves into a single
//! violin split down the middle. No violin-specific transform is involved.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    chart!(&penguins)?
        // One density curve per (Species, Sex) cell.
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupbys(["Species", "Sex"]),
        )?
        // Draw each curve as a half-band around its Species centre.
        .transform_band(
            BandTransform::new("Body Mass (g)", "density")
                .with_center("Species")
                .with_group("Sex")
                .with_split(true)
                .with_scale(BandScale::PerGroup),
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
            alt::path_group("path_group"),
            alt::color("Sex"),
        ))?
        .with_title("Body mass by species and sex (split)")
        .with_x_label("")
        .with_y_label("Body mass (g)")
        .save("docs/src/images/split_violin.svg")?;

    println!("Saved docs/src/images/split_violin.svg");
    Ok(())
}
