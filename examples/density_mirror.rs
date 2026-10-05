//! Mirrored density — a new picture recombined from the primitives.
//!
//! `mark_violin` pairs `transform_density` with `transform_band` (a mirrored
//! polygon). This example pairs the *same* statistic with `mark_area` and
//! `stack: "center"` instead: each curve is drawn from `-density / 2` to
//! `+density / 2`, then `coord_flip` stands it upright. It is the Vega-Lite
//! `density + area + stack` recipe, and it keeps the untrimmed tail rather than
//! stopping at the data like a violin.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_groupbys(["species"]),
        )?
        .mark_area()?
        .configure_area(|area| area.with_opacity(0.6).with_stroke("#2c3e50"))
        .encode((
            alt::x("sepal_length"),
            alt::y("density").with_stack("center"),
            alt::color("species"),
        ))?
        .coord_flip()
        .with_title("Mirrored density")
        .with_x_label("Density")
        .with_y_label("Sepal length (cm)")
        .save("target/example-output/density_mirror.svg")?;

    println!("Saved target/example-output/density_mirror.svg");
    Ok(())
}
