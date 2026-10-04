//! TEMPORARY: a violin outline with a *real* box plot on top.
//!
//! The standard violin recipe draws its inner summary with
//! `transform_quantile_box` — a plain inter-quartile rectangle plus a median
//! line. This example layers the full `mark_boxplot` (box + whiskers +
//! outliers) instead, and renders the two side by side for comparison.
//!
//! Output goes to `target/tmp/` so nothing lands in the repository.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    // Shared outline: density → symmetric band → polygon.
    let outline = || -> Result<Chart<MarkGeoPath>, charton::error::ChartonError> {
        chart!(&penguins)?
            .transform_density(
                DensityTransform::new("Body Mass (g)")
                    .with_as("Body Mass (g)", "density")
                    .with_groupbys(["Species"])
                    .with_trim(true),
            )?
            .transform_band(
                BandTransform::new("Body Mass (g)", "density")
                    .with_center("Species")
                    .with_scale(BandScale::PerGroup),
            )?
            .mark_polygon()?
            .configure_geoshape(|m| {
                m.with_fill("#d6eaf8")
                    .with_stroke("#2c3e50")
                    .with_stroke_width(1.0)
            })
            .encode((
                alt::x("x").with_category_labels("Species"),
                alt::y("y"),
                alt::path_group("path_group"),
            ))
    };

    // 1. Current recipe: outline + `transform_quantile_box` (rectangle + median).
    let quantile_box = chart!(&penguins)?
        .transform_quantile_box(
            QuantileBoxTransform::new("Body Mass (g)").with_category("Species"),
        )?
        .mark_polygon()?
        .configure_geoshape(|m| {
            m.with_fill("white")
                .with_stroke("black")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x").with_category_labels("Species"),
            alt::y("y"),
            alt::path_group("path_group"),
        ))?;

    outline()?
        .and(quantile_box)
        .with_title("Violin + transform_quantile_box (current recipe)")
        .with_x_label("")
        .with_y_label("Body mass (g)")
        .save("target/tmp/violin_quantile_box.png")?;

    // 2. This request: outline + a real `mark_boxplot` (box, whiskers, outliers).
    let real_box = chart!(&penguins)?
        .mark_boxplot()?
        .configure_boxplot(|m| {
            m.with_width(0.14)
                .with_color("white")
                .with_stroke("black")
                .with_stroke_width(1.0)
                .with_outlier_size(2.0)
        })
        .encode((alt::x("Species"), alt::y("Body Mass (g)")))?;

    outline()?
        .and(real_box)
        .with_title("Violin + mark_boxplot (real box, whiskers, outliers)")
        .with_x_label("")
        .with_y_label("Body mass (g)")
        .save("target/tmp/violin_real_box.png")?;

    println!("Saved target/tmp/violin_quantile_box.png and target/tmp/violin_real_box.png");
    Ok(())
}
