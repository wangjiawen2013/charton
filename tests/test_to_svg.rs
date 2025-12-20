use charton::prelude::*;
use polars::prelude::*;

#[test]
fn test_scatter_1() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! [
        "a" => [Some(130.0), None, Some(156.0), Some(1500.0), None],
        "b" => [-0.0001, -0.002, 0.001, 0.003, 1.0],
        "c" => ["USA", "USA", "Europe", "USA", "Japan"],
    ]?;

    // Create a point chart using the new API
    let point_chart = Chart::build(&df)?.mark_point().encode((x("a"), y("b")))?;

    // Create a layered chart and add the point chart as a layer
    LayeredChart::new()
        .with_size(500, 400)
        .add_layer(point_chart)
        .to_svg()?;

    Ok(())
}
