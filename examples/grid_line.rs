use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load the mtcars dataset
    let ds = load_dataset("mtcars")?;

    // Build a scatter plot
    Chart::build(ds)?
        .mark_point()?
        .configure_point(|p| p.with_size(30.0))
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear").with_scale(Scale::Discrete),
            alt::shape("gear").with_scale(Scale::Discrete),
            alt::size("mpg"),
        ))?
        .with_grid(true)
        .save("docs/src/images/grid_line.svg")?;

    Ok(())
}
