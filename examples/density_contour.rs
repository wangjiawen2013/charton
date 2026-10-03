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
//!
//! Each step **replaces the table**, so here is the column contract — read it,
//! then wire the emitted columns to channels in `encode`:
//!
//! | step | reads | emits |
//! |---|---|---|
//! | `transform_density_2d` | `sepal_length`, `petal_length` | `x`, `y`, `density` |
//! | `transform_contour` | `x`, `y`, `density` | `x`, `y`, `path_group`, `level` |
//!
//! `density` is consumed by the contour step (so it is not encoded); `path_group`
//! groups the vertices of one line and `level` gives it its value.

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
