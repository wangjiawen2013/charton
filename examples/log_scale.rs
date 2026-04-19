use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Example with GDP data that benefits from log scale
    let country = ["A", "B", "C", "D", "E"];
    let gdp = [1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0];
    let population = [100000.0, 500000.0, 2000000.0, 10000000.0, 50000000.0];

    chart!(country, gdp, population)?
        .mark_point()?
        .encode((
            alt::x("population"),
            alt::y("gdp").with_scale(Scale::Log), // Use logarithmic scale for GDP
        ))?
        .with_size(500, 400)
        .configure_theme(|t| t.with_x_tick_label_angle(-45.0))
        .coord_flip()
        .save("docs/src/images/log_scale.svg")?;

    Ok(())
}
