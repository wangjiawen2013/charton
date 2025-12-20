use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_errorbar_1() -> Result<(), Box<dyn Error>> {
    // Create sample data with multiple points having the same x values,
    // which allows calculating mean and standard deviation
    let df = df! [
        "x" => ["a", "a", "a", "b", "b", "b", "c", "c", "c", "d", "d", "d"],
        "y" => [5.1, 5.3, 5.7, 6.5, 6.9, 6.2, 4.0, 4.2, 4.4, 7.6, 8.0, 7.8],
    ]?;

    // Create error bar chart
    let errorbar_chart = Chart::build(&df)?
        .mark_errorbar()
        .with_errorbar_color(Some(SingleColor::new("blue")))
        .with_errorbar_stroke_width(2.0)
        .with_errorbar_cap_length(5.0)
        .with_errorbar_center(true) // Show center point
        .encode((x("x"), y("y")))?;

    // Create a layered chart and add the errorbar chart as a layer
    LayeredChart::new()
        .with_size(500, 400)
        .with_title("Error Bar Chart with Mean and Std Dev")
        .add_layer(errorbar_chart)
        .save("./tests/errorbar_1.svg")?;

    Ok(())
}
