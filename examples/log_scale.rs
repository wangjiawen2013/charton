use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Example with GDP data that benefits from log scale
    let df = df![
        "country" => ["A", "B", "C", "D", "E"],
        "gdp" => [1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0],
        "population" => [100000.0, 500000.0, 2000000.0, 10000000.0, 50000000.0]
    ]?;

    let point_chart = Chart::build(&df)?.mark_point().encode((
        x("population"),
        y("gdp").with_scale(Scale::Log), // Use logarithmic scale for GDP
    ))?;

    LayeredChart::new()
        .with_size(500, 400)
        .add_layer(point_chart)
        .save("./examples/log_scale.svg")?;

    Ok(())
}
