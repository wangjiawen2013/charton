use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_pie() -> Result<(), Box<dyn Error>> {
    // Create sample data frame for donut chart
    let df = df![
        "category" => ["A", "B", "C", "E", "D", "E"],
        "value" => [25.0, 30.0, 15.0, 30.0, 20.0, 10.0]
    ]?;

    // Create donut chart
    let donut = Chart::build(&df)?
        .mark_bar()?
        .encode((
            x(""),                 // x encoding for donut chart (empty string for donut chart)
            y("value"),            // theta encoding for donut slices
            color("category"),     // color encoding for different segments
        ))?;

    // Create a layered chart and add the donut chart as a layer
    LayeredChart::new()
        .add_layer(donut)
        .with_coord(CoordSystem::Polar)
        .save("./examples/pie.svg")?;

    Ok(())
}

#[test]
fn test_donut() -> Result<(), Box<dyn Error>> {
    // Create sample data frame for donut chart
    let df = df![
        "category" => ["A", "B", "C", "E", "D", "E"],
        "value" => [25.0, 30.0, 15.0, 30.0, 20.0, 10.0]
    ]?;

    // Create donut chart
    let donut = Chart::build(&df)?
        .mark_bar()?
        .encode((
            x(""),                 // x encoding for donut chart (empty string for donut chart)
            y("value"),            // theta encoding for donut slices
            color("category"),     // color encoding for different segments
        ))?;

    // Create a layered chart and add the donut chart as a layer
    LayeredChart::new()
        .add_layer(donut)
        .with_coord(CoordSystem::Polar)
        .with_inner_radius(0.5)  // Creates a donut chart
        .save("./examples/donut.svg")?;

    Ok(())
}

#[test]
fn test_rose() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let df = df! [
        "type" => ["a", "b", "c", "d"],
        "type2" => ["a", "b", "c", "d"],
        "value" => [4.9, 5.3, 5.5, 6.5],
        "value_std" => [0.3, 0.39, 0.34, 0.20]
    ]?;

    let bar = Chart::build(&df)?
        .mark_bar()?
        .encode((x("type"), y("value"), color("type")))?;

    // Create a layered chart and add the errorbar chart as a layer
    LayeredChart::new()
        .add_layer(bar)
        .with_y_label("value")
        .with_coord(CoordSystem::Polar)
        .save("./examples/rose.svg")?;

    Ok(())
}

#[test]
fn test_nightingale() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "Month" => ["Jan", "Jan", "Jan", "Jan", "Feb", "Feb", "Feb", "Feb", "Mar", "Mar", "Mar", "Mar"],
        "Revenue" => [500.0, 120.1, 90.0, 140.0, 110.0, 130.0, 100.0, 120.0, 90.0, 140.0, 110.0, 130.0],
        "Region" => ["North", "South", "East", "West", "North", "South", "East", "West", "North", "South", "East", "West"],
    ]?;

    // Create a bar chart with color encoding
    let colored_bar_chart = Chart::build(&df)?
        .mark_bar()?
        .encode((
            x("Month"),
            y("Revenue").with_stack(true).with_normalize(false),
            color("Region"),
        ))?;

    // Create a layered chart for colored bars
    LayeredChart::new()
        .with_title("Colored Bar Chart Example")
        .add_layer(colored_bar_chart)
        .with_coord(CoordSystem::Polar)
        .save("./examples/nightingale.svg")?;

    Ok(())
}