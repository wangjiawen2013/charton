use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data frame for donut chart
    let df = df![
        "category" => ["A", "B", "C", "E", "D", "E"],
        "value" => [25.0, 30.0, 15.0, 30.0, 20.0, 10.0]
    ]?;

    // Create donut chart
    let donut = Chart::build(&df)?
        .mark_bar()?
        .encode((
            x(""),                 // x encoding for donut chart (empty string for donut chart)
            y("value"),            // theta encoding for donut slices
            color("category"),     // color encoding for different segments
        ))?;

    // Create a layered chart and add the donut chart as a layer
    LayeredChart::new()
        .add_layer(donut)
        .with_coord(CoordSystem::Polar)
        .save("./examples/pie.svg")?;

    Ok(())
}