//! Density contour — iso-lines of a bivariate kernel density, with
//! `mark_contour`.
//!
//! The scattered points are first summarised into a density grid by the public
//! `transform_density_2d`, then `mark_contour("density")` turns that grid into
//! iso-lines and draws them. So the mark handles the contouring; the density
//! estimate stays an explicit, reusable step.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        // 1. Estimate the joint density of the two measurements.
        .transform_density_2d(
            // Pad the estimation grid so the outer iso-lines close before its
            // edge instead of being clipped by it.
            Density2DTransform::new("sepal_length", "petal_length")
                .with_grid_size(60)
                .with_padding(0.3),
        )?
        // 2. Extract and draw iso-density lines (coloured by level by default).
        .mark_contour("density")?
        .configure_contour(|contour| contour.with_levels(8).with_stroke_width(1.2))
        .encode((alt::x("x"), alt::y("y")))?
        .configure_theme(|t| t.with_color_map(ColorMap::Viridis))
        .with_title("Density contour")
        .with_x_label("Sepal length (cm)")
        .with_y_label("Petal length (cm)")
        .save("docs/src/images/density_contour.svg")?;

    println!("Saved docs/src/images/density_contour.svg");
    Ok(())
}
