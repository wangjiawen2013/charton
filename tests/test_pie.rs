use charton::prelude::*;
use std::error::Error;

#[test]
fn test_pie() -> Result<(), Box<dyn Error>> {
    // Create sample data frame for donut chart
    let category = ["A", "B", "C", "E", "D", "E"];
    let value = [25.0, 30.0, 15.0, 30.0, 20.0, 10.0];

    // Create donut chart
    chart!(value, category)?
        .mark_bar()?
        .encode((
            alt::x(""),             // x encoding for donut chart (empty string for donut chart)
            alt::y("value"),        // theta encoding for donut slices
            alt::color("category"), // color encoding for different segments
        ))?
        .with_coord(CoordSystem::Polar)
        .save("./tests/pie.svg")?;

    Ok(())
}

#[test]
fn test_donut() -> Result<(), Box<dyn Error>> {
    // Create sample data frame for donut chart
    let category = ["A", "B", "C", "E", "D", "E"];
    let value = [25.0, 30.0, 15.0, 30.0, 20.0, 10.0];

    // Create donut chart
    chart!(value, category)?
        .mark_bar()?
        .encode((
            alt::x(""),             // x encoding for donut chart (empty string for donut chart)
            alt::y("value"),        // theta encoding for donut slices
            alt::color("category"), // color encoding for different segments
        ))?
        .with_coord(CoordSystem::Polar)
        .with_inner_radius(0.5) // Creates a donut chart
        .save("./tests/donut.svg")?;

    Ok(())
}

#[test]
fn test_rose() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let type1 = ["a", "b", "c", "d"];
    let value = [4.9, 5.3, 5.5, 6.5];

    chart!(type1, value)?
        .mark_bar()?
        .encode((alt::x("type1"), alt::y("value"), alt::color("type1")))?
        .with_y_label("value")
        .with_coord(CoordSystem::Polar)
        .save("./tests/rose.svg")?;

    Ok(())
}

#[test]
fn test_nightingale() -> Result<(), Box<dyn Error>> {
    let month = [
        "Jan", "Jan", "Jan", "Jan", "Feb", "Feb", "Feb", "Feb", "Mar", "Mar", "Mar", "Mar",
    ];
    let revenue = [
        500.0, 120.1, 90.0, 140.0, 110.0, 130.0, 100.0, 120.0, 90.0, 140.0, 110.0, 130.0,
    ];
    let region = [
        "North", "South", "East", "West", "North", "South", "East", "West", "North", "South",
        "East", "West",
    ];

    // Create a bar chart with color encoding
    chart!(month, revenue, region)?
        .mark_bar()?
        .encode((
            alt::x("month"),
            alt::y("revenue")
                .with_stack("stacked")
                .with_normalize(false),
            alt::color("region"),
        ))?
        .with_title("Colored Bar Chart Example")
        .with_coord(CoordSystem::Polar)
        .save("./tests/nightingale.svg")?;

    Ok(())
}
