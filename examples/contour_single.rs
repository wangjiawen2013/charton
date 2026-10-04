//! Single-colour density contour, with `mark_contour`.
//!
//! The same iso-lines as `density_contour.rs`, but with the marks'
//! colour-by-level turned off, so every line uses the outline colour instead.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(&iris)?
        .transform_density_2d(
            // Pad the estimation grid so the outer iso-lines close before its
            // edge instead of being clipped by it.
            Density2DTransform::new("sepal_length", "petal_length")
                .with_grid_size(60)
                .with_padding(0.3),
        )?
        .mark_contour("density")?
        .configure_contour(|contour| {
            contour
                .with_levels(10)
                .with_color_by_level(false)
                .with_stroke("black")
                .with_stroke_width(1.0)
        })
        .encode((alt::x("x"), alt::y("y")))?
        .with_title("Single-colour density contour")
        .with_x_label("Sepal length (cm)")
        .with_y_label("Petal length (cm)")
        .save("docs/src/images/contour_single.svg")?;

    println!("Saved docs/src/images/contour_single.svg");
    Ok(())
}
