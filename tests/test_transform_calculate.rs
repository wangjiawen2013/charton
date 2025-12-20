use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_transform_calculate_1() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let df = df! [
        "type" => ["a", "b", "c", "d"],
        //"type" => [1.0, 2.5, 3.0, 4.1],
        "value" => [5.1, 5.3, 5.7, 6.5],
        "value_std" => [0.2, 0.23, 0.14, 0.25]
    ]?;

    // Create error bar chart using transform_calculate to add min/max values
    let errorbar_chart = Chart::build(&df)?
        // Use transform_calculate to create ymin and ymax columns based on fixed std values
        .transform_calculate(
            (col("value") - col("value_std")).alias("value_min"), // ymin = y - std
            (col("value") + col("value_std")).alias("value_max"), // ymax = y + std
        )?
        .mark_errorbar()
        .encode((x("type"), y("value_min"), y2("value_max")))?
        .swap_axes()
        .with_errorbar_color(Some(SingleColor::new("blue")))
        .with_errorbar_stroke_width(2.0)
        .with_errorbar_cap_length(5.0)
        .with_errorbar_center(true); // Show center point

    // Create a layered chart and add the errorbar chart as a layer
    LayeredChart::new()
        .with_size(500, 400)
        .with_title("Error Bar Chart with Mean and Std Dev")
        .add_layer(errorbar_chart)
        .save("./tests/transform_calculate_1.svg")?;

    Ok(())
}
