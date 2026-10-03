//! Styling ticks.
//!
//! `configure_tick` controls the visual weight of the marks: thickness, band
//! size and colour. This is useful for balancing a dense strip plot.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("iris")?;

    Chart::build(&ds)?
        .mark_tick()?
        .configure_tick(|m| {
            m.with_thickness(2.0)
                .with_band_size(10.0)
                .with_color("blue")
        })
        .encode((
            alt::x("sepal_width"),
            alt::y("species"),
            alt::color("species"),
        ))?
        .with_title("Styled ticks")
        .save("docs/src/images/tick_style.svg")?;

    println!("Saved docs/src/images/tick_style.svg");
    Ok(())
}
