use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let df = df! [
        "type" => ["a", "b", "c", "d"],
        "value" => [4.9, 5.3, 5.5, 6.5],
        "value_std" => [0.3, 0.39, 0.34, 0.20]
    ]?;

    // Create error bar chart using transform_calculate to add min/max values
    let errorbar = Chart::build(&df)?
        // Use transform_calculate to create ymin and ymax columns based on fixed std values
        .transform_calculate(
            (col("value") - col("value_std")).alias("value_min"), // ymin = y - std
            (col("value") + col("value_std")).alias("value_max"), // ymax = y + std
        )?
        .mark_errorbar()
        .encode((x("type"), y("value_min"), y2("value_max")))?;
    let bar = Chart::build(&df)?
        .mark_bar()
        .encode((x("type"), y("value")))?;

    // Create a layered chart and add the errorbar chart as a layer
    LayeredChart::new()
        .add_layer(errorbar)
        .add_layer(bar)
        .with_y_label("value")
        .save("./examples/bar_with_errorbar.svg")?;

    Ok(())
}
