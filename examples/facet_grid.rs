//! Grid faceting: arrange panels by two categorical dimensions.
//!
//! The built-in `mtcars` dataset contains observations for all four `vs x am`
//! combinations. Each panel therefore has real vehicle observations instead
//! of empty combinations produced by sparse categorical fields.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    Chart::build(ds)?
        .mark_point()?
        .configure_point(|point| point.with_size(4.0).with_opacity(0.85))
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear").with_scale(Scale::Discrete),
        ))?
        // vs: engine shape (0 = V-shaped, 1 = straight)
        // am: transmission (0 = automatic, 1 = manual)
        .facet(FacetSpec::grid("vs", "am").with_strategy("fixed"))
        .with_title("Car Performance by Engine Shape and Transmission")
        .with_size(1000, 800)
        .save("docs/src/images/facet_grid.svg")?;

    println!("Saved docs/src/images/facet_grid.svg");
    Ok(())
}
