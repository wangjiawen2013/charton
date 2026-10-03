//! Violin plot — the basic, single-distribution form, built by composition.
//!
//! A violin draws the *density* of a numeric column as a symmetric outline.
//! There is no violin mark and no violin-specific transform: the picture is the
//! same recipe Vega-Lite and Altair use, from three ordinary pieces:
//!
//! 1. `transform_density` estimates the density curve (the statistical part).
//! 2. `mark_area` with `stack: "center"` draws it mirrored around the centre
//!    (the geometric part).
//! 3. `coord_flip` stands the value axis upright.
//!
//! Grouping is a separate concern: add `-c` / `alt::color` and a `facet` (see
//! `grouped_violin.rs`), or a `Position` (see `grouped_violin.rs` part 2).

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        // 1. Estimate the density of sepal length.
        .transform_density(
            DensityTransform::new("sepal_length").with_as("sepal_length", "density"),
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
        .save("docs/src/images/violin.svg")?;

    println!("Saved docs/src/images/violin.svg");
    Ok(())
}
