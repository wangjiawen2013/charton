use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_rect_1() -> Result<(), Box<dyn Error>> {
    // Sample data for heatmap
    let df = df! [
        "a" => ["A", "B", "C", "A", "B", "C", "A", "B", "C"],
        "b" => ["X", "X", "X", "Y", "Y", "Y", "Z", "Z", "Z"],
        "value" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
    ]?;

    // Create heatmap chart
    Chart::build(&df)?
        .mark_rect()?
        .encode((x("a"), y("b"), color("value")))?
        .with_size(500, 400)
        .save("./tests/rect_1.svg")?;

    Ok(())
}

#[test]
fn test_rect_2() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "a" => [1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4],
        "b" => [1, 2, 1, 2, 3, 1, 2, 3, 1, 2, 3],
        "value" => [1.0, 2.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
    ]?;

    // Create heatmap chart
    Chart::build(&df)?
        .mark_rect()?
        .encode((x("a"), y("b"), color("value")))?
        .with_size(500, 400)
        .coord_flip()
        .save("./tests/rect_2.svg")?;

    Ok(())
}

#[test]
fn test_rect_3() -> Result<(), Box<dyn Error>> {
    // Sample data for heatmap with continuous variables
    let df = df! [
        "x" => [1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.8,2.05, 2.2, 2.5, 2.6, 2.7],
        "y" => [1.2, 1.3, 1.4, 1.5, 1.8, 1.83, 2.0, 1.9, 2.2, 2.3, 2.4, 2.5],
        "value" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
    ]?;
    // Create heatmap chart
    Chart::build(&df)?
        .mark_rect()?
        .encode((x("x"), y("y"), color("value")))?
        .with_size(500, 400)
        .configure_theme(|t| t.with_color_map(ColorMap::GnBu))
        .save("./tests/rect_3.svg")?;

    Ok(())
}
