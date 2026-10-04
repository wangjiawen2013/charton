//! Violin plot — built by hand from the primitives.
//!
//! This is the same picture as `violin.rs`, written without `mark_violin` so the
//! composition is visible. It is the reference for anyone who wants to combine
//! the basic parts differently. A violin draws the *density* of a numeric column
//! as a symmetric outline, from three ordinary pieces:
//!
//! 1. `transform_density` estimates the density curve (the statistical part).
//! 2. `mark_area` with `stack: "center"` draws it mirrored around the centre
//!    (the geometric part).
//! 3. `coord_flip` stands the value axis upright.
//!
//! Grouping is a separate concern: add `alt::color` and a `facet` for one panel
//! per group, or `transform_band(…).with_split(true)` for a split violin.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        // 1. Estimate the density of sepal length.
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                // Trim to the observed range: a violin ends at the data, it does
                // not fade out into a thin tail (ggplot2 `trim = TRUE`).
                .with_trim(true),
        )?
        // 2. Draw it as an area, mirrored around zero.
        .mark_area()?
        .configure_area(|a| {
            a.with_color("#7fb3d5")
                .with_opacity(0.6)
                .with_stroke("#2c3e50")
        })
        .encode((
            alt::x("sepal_length"),
            alt::y("density").with_stack("center"),
        ))?
        // 3. Value on the vertical axis, density on the horizontal one.
        .coord_flip()
        .with_title("Sepal length")
        .with_x_label("Density")
        .with_y_label("Sepal length (cm)")
        .save("docs/src/images/violin_manual.svg")?;

    println!("Saved docs/src/images/violin_manual.svg");
    Ok(())
}
