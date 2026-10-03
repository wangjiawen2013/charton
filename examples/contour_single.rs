//! Single-colour contour.
//!
//! The same iso-lines as `contour.rs`, but with no colour channel: the path's
//! own stroke is used, so every line is one colour.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        .transform_density_2d(
            // Pad the estimation grid so the outer iso-lines close before its
            // edge instead of being clipped by it.
            Density2DTransform::new("sepal_length", "petal_length")
                .with_grid_size(60)
                .with_padding(0.3),
        )?
        .transform_contour(ContourTransform::new("x", "y", "density").with_levels(10))?
        .mark_path()?
        .configure_path(|m| m.with_stroke("black").with_stroke_width(1.0))
        .encode((alt::x("x"), alt::y("y"), alt::path_group("path_group")))?
        .with_title("Single-colour density contour")
        .with_x_label("Sepal length (cm)")
        .with_y_label("Petal length (cm)")
        .save("docs/src/images/contour_single.svg")?;

    println!("Saved docs/src/images/contour_single.svg");
    Ok(())
}
