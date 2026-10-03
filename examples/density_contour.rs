//! Density contour — iso-lines of a bivariate kernel density.
//!
//! The scattered points are first summarised into a density grid
//! (`transform_density_2d`), then that grid is turned into iso-lines
//! (`transform_contour`) and drawn as open paths (`mark_path`). Three general
//! pieces, no density-contour-specific mark:
//!
//! ```text
//! scatter  →  2D density grid  →  iso-lines  →  path
//! ```

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        // 1. Estimate the joint density of the two measurements.
        .transform_density_2d(
            Density2DTransform::new("sepal_length", "petal_length").with_grid_size(60),
        )?
        // 2. Extract iso-density lines from the grid.
        .transform_contour(ContourTransform::new("x", "y", "density").with_levels(8))?
        // 3. Draw them.
        .mark_path()?
        .configure_path(|m| m.with_stroke_width(1.2))
        .encode((
            alt::x("x"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("level"),
        ))?
        .configure_theme(|t| t.with_color_map(ColorMap::Viridis))
        .with_title("Density contour")
        .with_x_label("Sepal length (cm)")
        .with_y_label("Petal length (cm)")
        .save("docs/src/images/density_contour.svg")?;

    println!("Saved docs/src/images/density_contour.svg");
    Ok(())
}
