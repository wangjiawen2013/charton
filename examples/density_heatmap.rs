//! Density heatmap — a bivariate kernel density drawn as a grid of cells.
//!
//! `transform_density_2d` estimates the joint density of two numeric columns
//! onto a regular grid; `mark_rect` then paints each cell with its density.
//! This is the filled counterpart of the density contour in
//! `density_contour.rs`: same grid, cells instead of iso-lines.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        // 1. Estimate the joint density on a regular grid.
        .transform_density_2d(
            Density2DTransform::new("sepal_length", "petal_length")
                .with_grid_size(60)
                .with_padding(0.2),
        )?
        // 2. Paint one cell per grid node. Match the bin count to the grid so
        //    the cells are not merged back together.
        .mark_rect()?
        .encode((
            alt::x("x").with_bins(60),
            alt::y("y").with_bins(60),
            alt::color("density"),
        ))?
        .configure_theme(|t| t.with_color_map(ColorMap::Viridis))
        .with_title("Density heatmap")
        .with_x_label("Sepal length (cm)")
        .with_y_label("Petal length (cm)")
        .save("docs/src/images/density_heatmap.svg")?;

    println!("Saved docs/src/images/density_heatmap.svg");
    Ok(())
}
