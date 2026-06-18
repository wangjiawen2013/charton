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
            //alt::size("mpg"),
        ))?
        .coord_flip()
        .configure_theme(|t| t.with_x_tick_label_angle(-45.0))
        .with_title("Car Performance")
        .with_grid(true)
        .save("scatter_gpu.png")?;

    println!("Success! Scatter plot saved as 'scatter_gpu.png' using wgpu.");
    Ok(())
}
