use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Sample data for heatmap
    let df = df! [
        "x" => ["A", "B", "C", "A", "B", "C", "A", "B", "C"],
        "y" => ["X", "X", "X", "Y", "Y", "Y", "Z", "Z", "Z"],
        "value" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
    ]?;

    // Create heatmap chart
    let rect_chart = Chart::build(&df)?
        .mark_rect()
        .encode((x("x"), y("y"), color("value")))?;

    // Create a layered chart and add the rect chart as a layer
    LayeredChart::new()
        .add_layer(rect_chart)
        .save("./examples/heatmap.svg")?;

    Ok(())
}
