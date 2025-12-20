use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Sample data for heatmap with continuous variables
    let df = df! [
        "x" => [1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.8,2.05, 2.2, 2.5, 2.6, 2.7],
        "y" => [1.2, 1.3, 1.4, 1.5, 1.8, 1.83, 2.0, 1.9, 2.2, 2.3, 2.4, 2.5],
        "value" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
    ]?;
    // Create heatmap chart
    let rect_chart = Chart::build(&df)?
        .mark_rect()
        .encode((x("x"), y("y"), color("value")))?
        .with_color_map(ColorMap::GnBu);

    // Create a layered chart and add the rect chart as a layer
    LayeredChart::new()
        .add_layer(rect_chart)
        .save("./examples/2d_density.svg")?;

    Ok(())
}
