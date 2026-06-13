use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load the mtcars dataset
    let ds = load_dataset("mtcars")?;

    // Build a scatter plot
    Chart::build(ds)?
        .mark_point()?
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear").with_scale(Scale::Discrete),
            alt::shape("gear").with_scale(Scale::Discrete),
            alt::size("mpg"),
        ))?
        .coord_flip()
        .configure_theme(|t| t.with_x_tick_label_angle(-45.0))
        .with_title("Car Performance")
        // The save() method automatically detects the 'wgpu' feature
        // and uses the high-performance GPU backend to generate this PNG.
        .save("scatter_gpu.png")?;

    println!("Success! Scatter plot saved as 'scatter_gpu.png' using wgpu.");
    Ok(())
}
