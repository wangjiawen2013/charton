use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_scatter_1() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "a" => [Some(130.0), None, Some(156.0), Some(1500.0), None],
        "b" => [-0.0001, -0.002, 0.001, 0.003, 1.0],
        "c" => ["USA", "USA", "Europe", "USA", "Japan"],
    ]?;
    println!("{}", df);

    // Create a point chart using the new API
    let point_chart = Chart::build(&df)?
        .mark_point()
        .encode((
            x("a").with_scale(Scale::Linear),
            y("b").with_scale(Scale::Linear),
        ))?
        .with_point_stroke_width(1.0)
        .with_point_stroke(Some(SingleColor::new("black")))
        .with_point_color(Some(SingleColor::new("red")))
        .swap_axes();

    // Create a layered chart and add the point chart as a layer
    LayeredChart::new()
        .with_size(500, 400)
        .add_layer(point_chart)
        .save("./tests/scatter_1.svg")?;

    Ok(())
}

#[test]
fn test_scatter_2() -> Result<(), Box<dyn Error>> {
    let df = df![
        "a" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0],
        "b" => [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 110.0, 120.0, 130.0, 140.0, 150.0, 160.0, 170.0, 180.0],
        "category" => ["A123XY", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R"]
    ]?;

    let point_chart = Chart::build(&df)?.mark_point().encode((
        x("a"),
        y("b"),
        shape("category"),
        //color("category"),
    ))?;

    LayeredChart::new()
        .with_size(500, 300)
        .add_layer(point_chart)
        .save("./tests/scatter_2.svg")?;

    Ok(())
}

#[test]
fn test_scatter_3() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "a" => [130.0, -165.0, 156.0, -150.0, 1400.0],
        "b" => [-0.0001, -0.002, 0.001, 0.003, 1.0],
        "Origin" => ["USA", "USA", "Europe", "USA", "Japan"],
    ]?;

    let point_chart = Chart::build(&df)?
        .mark_point()
        .encode((x("a"), y("b"), color("Origin")))?;

    LayeredChart::new()
        .with_size(500, 300)
        .with_title("Data")
        .with_x_label("A")
        .with_y_label("B")
        .add_layer(point_chart)
        .save("./tests/scatter_3.svg")?;

    Ok(())
}

#[test]
fn test_scatter_4() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "a" => [130.0, 165.0, 156.0, 150.0],
        "b" => [18.0, 15.0, 20.0, 16.0],
        "Origin" => ["USA", "USA", "Europe", "Japan"],
    ]?;

    let point_chart = Chart::build(&df)?
        .mark_point()
        .encode((x("a"), y("b"), color("Origin")))?;

    LayeredChart::new()
        .with_size(500, 300)
        .with_title("Car Data")
        .with_title_font_size(20)
        .with_title_color("#333")
        .add_layer(point_chart)
        .save("./tests/scatter_4.svg")?;

    Ok(())
}

#[test]
fn test_scatter_5() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "a" => [130.0, 165.0, 156.0, 1500.0],
        "b" => [18.0, 15.0, 20.0, 16.0],
        "Origin" => ["USA", "USA", "Europe", "Japan"],
    ]?;

    let point_chart = Chart::build(&df)?
        .mark_point()
        .encode((x("a"), y("b"), color("Origin")))?
        .swap_axes();

    LayeredChart::new()
        .with_size(500, 400)
        .with_title("Data")
        .with_title_font_size(20)
        .with_title_color("#333")
        .with_x_label("A)")
        .with_y_tick_label_angle(-45.0)
        .with_y_label("B")
        .with_label_color("steelblue")
        .with_label_font_family("serif")
        .with_label_font_size(36)
        .add_layer(point_chart)
        .save("./tests/scatter_5.svg")?;

    Ok(())
}

#[test]
fn test_scatter_6() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "a" => &[130.0, 165.0, 150.0, 150.0, 225.0, 97.0],
        "b" => &[18.0, 15.0, 18.0, 16.0, 17.0, 30.0],
        "Origin" => &["USA", "Germany", "Japan", "USA", "Germany", "Japan"],
    ]?;

    let point_chart = Chart::build(&df)?
        .mark_point()
        .encode((x("a"), y("b"), color("Origin")))?;

    LayeredChart::new()
        .with_size(500, 400)
        .with_title("Standard Chart: A vs B")
        .with_x_label("A")
        .with_y_label("B")
        .add_layer(point_chart)
        .save("./tests/scatter_6.svg")?;

    Ok(())
}

#[test]
fn test_scatter_7() -> Result<(), Box<dyn Error>> {
    let df = df![
        "a" => [1.0, 2.0, 3.0, 4.0, 5.0, 1.5, 2.5, 3.5, 4.5, 5.5],
        "b" => [2.0, 4.0, 1.0, 5.0, 3.0, 3.5, 1.5, 4.5, 2.5, 5.5],
        "category" => ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"],
        "value" => [10.0, 20.0, 15.0, 25.0, 12.0, 18.0, 22.0, 14.0, 28.0, 16.0],
        "confidence" => [0.9, 0.7, 0.8, 0.6, 0.95, 0.85, 0.75, 0.88, 0.72, 0.92]
    ]?;

    let point_chart = Chart::build(&df)?.mark_point().encode((
        x("a"),
        y("b"),
        //shape("category"),
        //size("value"),
        //fill("category"),
        //opacity("confidence"),
        color("category"),
    ))?;

    LayeredChart::new()
        .with_size(500, 300)
        .with_title("visualization")
        .add_layer(point_chart)
        .save("./tests/scatter_7.svg")?;

    Ok(())
}

#[test]
fn test_scatter_8() -> Result<(), Box<dyn Error>> {
    // Example with GDP data that benefits from log scale
    let df = df![
        "country" => ["A", "B", "C", "D", "E"],
        "gdp" => [1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0],
        "population" => [100000.0, 500000.0, 2000000.0, 10000000.0, 50000000.0]
    ]?;

    let point_chart = Chart::build(&df)?.mark_point().encode((
        x("population"),
        y("gdp").with_scale(Scale::Log), // Use logarithmic scale for GDP
    ))?;

    LayeredChart::new()
        .with_size(500, 400)
        .add_layer(point_chart)
        .save("./tests/scatter_8.svg")?;

    Ok(())
}

#[test]
fn test_scatter_9() -> Result<(), Box<dyn Error>> {
    // Example with categorical data on x-axis and numerical data on y-axis
    let df = df![
        "department" => ["Engineering", "Marketing", "Sales", "HR", "Finance", "Engineering", "Marketing"],
        "salary" => [85000.0, 65000.0, 60000.0, 55000.0, 75000.0, 90000.0, 68000.0]
    ]?;

    // Create a point chart using the new API
    let point_chart = Chart::build(&df)?
        .mark_point() // This is the correct method name
        .encode((
            x("department"), // Discrete categorical data on x-axis
            y("salary"),     // Continuous numerical data on y-axis
        ))?;

    // Create a layered chart and add the point chart as a layer
    LayeredChart::new()
        .with_size(600, 400)
        .with_title("Salary by Department")
        .add_layer(point_chart)
        .save("./tests/scatter_9.svg")?;

    Ok(())
}

#[test]
fn test_scatter_10() -> Result<(), Box<dyn Error>> {
    // Create sample data with a continuous variable for color encoding
    let df = df![
        "a" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
        "b" => [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0],
        "value" => [1.5, 2.3, 3.7, 4.1, 5.9, 6.2, 7.8, 8.4, 9.1, 10.6]  // Continuous variable for color
    ]?;

    // Create a chart with color encoding using the continuous variable
    let chart = Chart::build(&df)?.mark_point().encode((
        x("a"),
        y("b"),
        color("value"), // This will trigger the colorbar instead of discrete legend
        size("value"),  // This will trigger the colorbar instead of discrete legend
    ))?;

    LayeredChart::new()
        .with_size(500, 300)
        .with_title("Chart with Colorbar")
        .add_layer(chart)
        .save("./tests/scatter_10.svg")?;

    Ok(())
}

#[test]
fn test_scatter_11() -> Result<(), Box<dyn Error>> {
    // Create sample data with a categorical variable for shape encoding
    let df = df![
        "a" => [1.0, 2.0, 3.0, 4.0, 5.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        "b" => [10.0, 20.0, 30.0, 40.0, 50.0, 15.0, 25.0, 35.0, 45.0, 55.0],
        "category" => ["A", "B", "C", "A", "B", "C", "A", "B", "C", "A"]  // Categorical variable for shape
    ]?;

    // Create a chart with shape encoding using the categorical variable
    let chart = Chart::build(&df)?.mark_point().encode((
        x("a"),
        y("b"),
        //size("x"),
        shape("category"), // This will trigger the shape legend
        color("category"), // This will trigger the shape legend
    ))?;

    LayeredChart::new()
        .with_size(500, 300)
        .with_title("Chart with Shape Legend")
        .add_layer(chart)
        .save("./tests/scatter_11.svg")?;

    Ok(())
}

#[test]
fn test_scatter_12() -> Result<(), Box<dyn Error>> {
    let df = df![
        "a" => [1.0, 2.0, 3.0, 4.0, 5.0],
        "b" => [10.0, 20.0, 30.0, 40.0, 50.0],
        "category" => ["A", "B", "A", "B", "C"]
    ]?;

    // Create a point chart with only x, y, and color encodings
    let chart = Chart::build(&df)?
        .mark_point()
        .encode((x("a"), y("b"), color("category")))?;

    LayeredChart::new()
        .with_size(500, 300)
        .add_layer(chart)
        .save("./tests/scatter_12.svg")?;

    Ok(())
}

#[test]
fn test_scatter_13() -> Result<(), Box<dyn Error>> {
    // Create sample data
    let df = df![
        "a" => [1.0, 2.0, 3.0, 4.0, 5.0],
        "b" => [10.0, 20.0, 30.0, 40.0, 50.0],
        "category" => ["A", "B", "C", "D", "E"]
    ]?;

    // Create a chart with explicit tick values using the new API
    let point_chart =
        Chart::build(&df)?
            .mark_point()
            .encode((x("category"), y("b"), color("category")))?;

    // Create a layered chart and add the point chart as a layer
    LayeredChart::new()
        .with_size(600, 400)
        .with_title("Chart with Explicit Tick Values")
        .add_layer(point_chart)
        .with_x_label("Catergory")
        //.with_x_tick_values(vec![1.0, 2.0, 3.0, 4.0, 5.0])
        .with_x_tick_labels(vec!["A", "D", "F"])
        .with_y_label("B Values")
        .with_y_tick_values(vec![0.0, 10.1, 5.0])
        .save("./tests/scatter_13.svg")?;

    Ok(())
}

#[test]
fn test_scatter_14() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "a" => [130.0, 165.0, 156.0, 1500.0],
        "b" => [18.0, 15.0, 20.0, 16.0],
        "Origin" => ["USA", "USA", "Europe", "Japan"],
    ]?;

    Chart::build(&df)?
        .mark_point()
        .encode((x("a"), y("b"), color("Origin")))?
        .into_layered()
        .save("./tests/scatter_14.svg")?;

    Ok(())
}
