use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_empty_1() -> Result<(), Box<dyn Error>> {
    let df = df![
        "a" => Vec::<f64>::new(),
        "b" => Vec::<f64>::new()
    ]?;

    // Create a chart with empty data
    let empty_chart = Chart::build(&df)?
        .mark_point()
        .encode((
            x("a").with_scale(Scale::Linear),
            y("b").with_scale(Scale::Linear),
        ))?
        .with_point_stroke_width(1.0)
        .with_point_stroke(Some(SingleColor::new("black")))
        .with_point_color(Some(SingleColor::new("red")));

    // Create a layered chart and add the layer
    LayeredChart::new()
        .with_size(500, 400)
        .add_layer(empty_chart)
        .swap_axes()
        .save("./tests/empty_1.svg")?;

    Ok(())
}

#[test]
fn test_empty_2() -> Result<(), Box<dyn Error>> {
    let df_empty = df![
        "a" => Vec::<f64>::new(),
        "b" => Vec::<f64>::new()
    ]?;
    let empty_chart = Chart::build(&df_empty)?.mark_point().encode((
        x("a").with_scale(Scale::Linear),
        y("b").with_scale(Scale::Linear),
    ))?;

    let df = df![
        "a" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0],
        "b" => [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 110.0, 120.0, 130.0, 140.0, 150.0, 160.0, 170.0, 180.0],
        "category" => ["A123XY", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R"]
    ]?;

    let point_chart =
        Chart::build(&df)?
            .mark_point()
            .encode((x("a"), y("b"), shape("category")))?;

    LayeredChart::new()
        .with_size(500, 300)
        .add_layer(point_chart)
        .add_layer(empty_chart)
        .save("./tests/empty_2.svg")?;

    Ok(())
}
