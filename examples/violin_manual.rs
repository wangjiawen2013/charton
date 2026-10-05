//! Violin plot — built by hand from the primitives.
//!
//! This is the same picture as `violin.rs`, written without `mark_violin` so the
//! composition is visible. It is the reference for anyone who wants to combine
//! the basic parts differently. A violin draws the *density* of a numeric column
//! as a symmetric outline, from three ordinary pieces:
//!
//! 1. `transform_density` estimates the density curve (the statistical part);
//! 2. `transform_band` mirrors it around the centre into a polygon;
//! 3. `mark_polygon` draws the closed, filled outline.
//!
//! Grouping is a separate concern: add `alt::x` for one violin per category,
//! `alt::color` to sit several side by side (dodged), or
//! `transform_band(…).with_split(true)` for a split violin. This is exactly what
//! `mark_violin` expands to, so the two render byte for byte identically.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        // 1. Estimate the density of sepal length. Trim to the observed range:
        //    a violin ends at the data, it does not fade into a thin tail.
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_trim(true),
        )?
        // 2. Mirror the curve around the centre and emit a polygon.
        .transform_band(BandTransform::new("sepal_length", "density"))?
        // 3. Fill and outline the closed shape.
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("#7fb3d5")
                .with_opacity(0.6)
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x"),
            alt::y("y").with_label("sepal_length"),
            alt::path_group("path_group"),
        ))?
        .with_title("Sepal length")
        .with_x_label("Density")
        .with_y_label("Sepal length (cm)")
        .save("target/example-output/violin_manual.svg")?;

    println!("Saved target/example-output/violin_manual.svg");
    Ok(())
}
