//! Density plot — built by hand from the primitives.
//!
//! The same picture as `density.rs`, without `mark_density`. A one-dimensional
//! density estimate (a KDE) draws the shape of a numeric column as a smooth
//! curve: `transform_density` estimates the curve and `mark_area` draws it.
//!
//! ```text
//! transform_density   // numeric column  →  (value, density)
//!   → mark_area       // draw the curve
//! ```

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("iris")?;

    chart!(ds)?
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_groupbys(["species"]),
        )?
        .mark_area()?
        .configure_area(|a| a.with_opacity(0.5))
        .encode((
            alt::x("sepal_length"),
            alt::y("density"),
            alt::color("species"),
        ))?
        .save("docs/src/images/density_manual.svg")?;

    Ok(())
}
