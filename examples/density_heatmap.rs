//! Density heatmap — a bivariate kernel density drawn as a grid of cells, with
//! `mark_density_2d`.
//!
//! `mark_density_2d` is a **composite** mark: it expands into the public
//! `transform_density_2d` and the shared rectangle geometry. It sets the heatmap
//! bin count to the grid size automatically, so the caller does not have to know
//! that `mark_rect` bins a second time. This is the filled counterpart of the
//! density contour.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        .mark_density_2d()?
        .configure_density_2d(|density| density.with_grid_size(60).with_padding(0.2))
        .encode((alt::x("sepal_length"), alt::y("petal_length")))?
        .configure_theme(|t| t.with_color_map(ColorMap::Viridis))
        .with_title("Density heatmap")
        .with_x_label("Sepal length (cm)")
        .with_y_label("Petal length (cm)")
        .save("docs/src/images/density_heatmap.svg")?;

    println!("Saved docs/src/images/density_heatmap.svg");
    Ok(())
}
