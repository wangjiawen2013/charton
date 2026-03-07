use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let df = df! [
        "type" => ["a", "a", "a", "a", "b", "b", "b", "b", "c", "c", "c", "c"],
        "value" => [4.1, 5.3, 5.5, 6.5, 4.2, 5.1, 5.7, 5.5, 4.3, 5.5, 5.1, 6.8],
        "value_std" => [0.22, 0.26, 0.14, 0.23, 0.2, 0.23, 0.12, 0.25, 0.21, 0.20, 0.16, 0.25],
        "group" => ["E", "F", "G", "H", "E", "F", "G", "H", "E", "F", "G", "H"]
    ]?;

    // Create error bar chart using transform_calculate to add min/max values
    let errorbar = Chart::build(&df)?
        // Use transform_calculate to create ymin and ymax columns based on fixed std values
        .transform_calculate(
            (col("value") - col("value_std")).alias("value_min"), // ymin = y - std
            (col("value") + col("value_std")).alias("value_max"), // ymax = y + std
        )?
        .mark_errorbar()?
        .encode((x("type"), y("value_min"), y2("value_max"), color("group")))?;

    // Create a bar chart
    let bar = Chart::build(&df)?
        .mark_bar()?
        .encode((x("type"), y("value"), color("group")))?;

    // Create a layered chart and add the errorbar chart as a layer
    LayeredChart::new()
        .add_layer(errorbar)
        .add_layer(bar)
        .save("./examples/grouped_bar_with_errorbar_2.svg")?;

    Ok(())
}
