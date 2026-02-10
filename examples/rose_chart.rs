use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let df = df! [
        "type" => ["a", "b", "c", "d"],
        "type2" => ["a", "b", "c", "d"],
        "value" => [4.9, 5.3, 5.5, 6.5],
        "value_std" => [0.3, 0.39, 0.34, 0.20]
    ]?;

    let bar = Chart::build(&df)?
        .mark_bar()
        .encode((x("type"), y("value"), color("type")))?;

    // Create a layered chart and add the errorbar chart as a layer
    LayeredChart::new()
        .add_layer(bar)
        .with_y_label("value")
        .with_coord(CoordSystem::Polar)
        .save("./examples/rose_chart.svg")?;

    Ok(())
}