use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_rule_1() -> Result<(), Box<dyn Error>> {
    // Create sample data with x, y, y2, and color columns
    let df = df![
        "x" => [1.0, 2.0, 3.0, 4.0, 5.0],
        "y" => [2.0, 3.0, 1.0, 4.0, 2.0],
        "y2" => [4.0, 5.0, 3.0, 6.0, 4.0],
        "color" => ["A", "B", "A", "B", "A"]
    ]?;

    // Create a chart with rule marks
    let chart = Chart::build(&df)?
        .mark_rule()
        .encode((x("x"), y("y"), y2("y2"), color("color")))?
        .into_layered()
        .with_title("Rule Chart with Y and Y2")
        .with_x_label("X Values")
        .with_y_label("Y Values");

    // Save to SVG file
    chart.save("tests/rule_1.svg")?;

    Ok(())
}
