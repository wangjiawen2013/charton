use charton::prelude::*;
use std::error::Error;

#[test]
fn test_scatter_1() -> Result<(), Box<dyn Error>> {
    let a = [Some(130.0), None, Some(156.0), Some(1500.0), None];
    let b = [-0.0001, -0.002, 0.001, 0.003, 1.0];
    let c = ["USA", "USA", "Europe", "USA", "Japan"];

    // Create a point chart using the new API
    let point_chart = chart!(a, b, c)?
        .mark_point()?
        .configure_point(|p| {
            p.with_stroke_width(1.0)
                .with_stroke("black")
                .with_color("red")
        })
        .encode((
            alt::x("a").with_scale(Scale::Linear),
            alt::y("b").with_scale(Scale::Linear),
        ))?;

    // Create a layered chart and add the point chart as a layer
    point_chart
        .with_size(500, 400)
        .coord_flip()
        .save("./tests/scatter_1.svg")?;

    Ok(())
}

#[test]
fn test_scatter_2() -> Result<(), Box<dyn Error>> {
    let a = [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0,
    ];
    let b = [
        10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 110.0, 120.0, 130.0, 140.0,
        150.0, 160.0, 170.0, 180.0,
    ];
    let category = [
        "A123XY", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q",
        "R",
    ];

    chart!(a, b, category)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::shape("category")))?
        .with_size(500, 300)
        .save("./tests/scatter_2.svg")?;

    Ok(())
}

#[test]
fn test_scatter_3() -> Result<(), Box<dyn Error>> {
    let a = [130.0, -165.0, 156.0, -150.0, 1400.0];
    let b = [-0.0001, -0.002, 0.001, 0.003, 1.0];
    let origin = ["USA", "USA", "Europe", "USA", "Japan"];

    chart!(a, b, origin)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("origin")))?
        .with_size(500, 300)
        .with_title("Data")
        .with_x_label("A")
        .with_y_label("B")
        .save("./tests/scatter_3.svg")?;

    Ok(())
}

#[test]
fn test_scatter_4() -> Result<(), Box<dyn Error>> {
    let a = [130.0, 165.0, 156.0, 150.0];
    let b = [18.0, 15.0, 20.0, 16.0];
    let origin = ["USA", "USA", "Europe", "Japan"];

    chart!(a, b, origin)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("origin")))?
        .with_size(500, 300)
        .with_title("Car Data")
        .configure_theme(|t| t.with_title_size(20.0).with_title_color("#333"))
        .save("./tests/scatter_4.svg")?;

    Ok(())
}

#[test]
fn test_scatter_5() -> Result<(), Box<dyn Error>> {
    let a = [130.0, 165.0, 156.0, 1500.0];
    let b = [18.0, 15.0, 20.0, 16.0];
    let origin = ["USA", "USA", "Europe", "Japan"];

    chart!(a, b, origin)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("origin")))?
        .with_size(500, 400)
        .with_title("Data")
        .with_x_label("A)")
        .with_y_label("B")
        .configure_theme(|t| {
            t.with_title_size(20.0)
                .with_title_color("#333")
                .with_y_tick_label_angle(-45.0)
                .with_label_color("steelblue")
                .with_label_family("serif")
                .with_label_size(36.0)
        })
        .coord_flip()
        .save("./tests/scatter_5.svg")?;

    Ok(())
}

#[test]
fn test_scatter_6() -> Result<(), Box<dyn Error>> {
    let a = &[130.0, 165.0, 150.0, 150.0, 225.0, 97.0];
    let b = &[18.0, 15.0, 18.0, 16.0, 17.0, 30.0];
    let origin = &["USA", "Germany", "Japan", "USA", "Germany", "Japan"];

    let point_chart = chart!(a, b, origin)?.mark_point()?.encode((
        alt::x("a"),
        alt::y("b"),
        alt::color("origin"),
    ))?;

    point_chart
        .with_size(500, 400)
        .with_title("Standard Chart: A vs B")
        .with_x_label("A")
        .with_y_label("B")
        .save("./tests/scatter_6.svg")?;

    Ok(())
}

#[test]
fn test_scatter_7() -> Result<(), Box<dyn Error>> {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0, 1.5, 2.5, 3.5, 4.5, 5.5];
    let b = [2.0, 4.0, 1.0, 5.0, 3.0, 3.5, 1.5, 4.5, 2.5, 5.5];
    let category = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];
    let value = [10.0, 20.0, 15.0, 25.0, 12.0, 18.0, 22.0, 14.0, 28.0, 16.0];
    let confidence = [0.9, 0.7, 0.8, 0.6, 0.95, 0.85, 0.75, 0.88, 0.72, 0.92];

    chart!(a, b, category, value, confidence)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("category")))?
        .with_size(500, 300)
        .with_title("visualization")
        .save("./tests/scatter_7.svg")?;

    Ok(())
}

#[test]
fn test_scatter_8() -> Result<(), Box<dyn Error>> {
    // Example with GDP data that benefits from log scale
    let country = ["A", "B", "C", "D", "E"];
    let gdp = [1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0];
    let population = [100000.0, 500000.0, 2000000.0, 10000000.0, 50000000.0];

    chart!(country, gdp, population)?
        .mark_point()?
        .encode((
            alt::x("population"),
            alt::y("gdp").with_scale(Scale::Log), // Use logarithmic scale for GDP
        ))?
        .with_size(500, 400)
        .save("./tests/scatter_8.svg")?;

    Ok(())
}

#[test]
fn test_scatter_9() -> Result<(), Box<dyn Error>> {
    // Example with categorical data on x-axis and numerical data on y-axis
    let department = [
        "Engineering",
        "Marketing",
        "Sales",
        "HR",
        "Finance",
        "Engineering",
        "Marketing",
    ];
    let salary = [
        85000.0, 65000.0, 60000.0, 55000.0, 75000.0, 90000.0, 68000.0,
    ];

    // Create a point chart
    chart!(department, salary)?
        .mark_point()?
        .encode((alt::x("department"), alt::y("salary")))?
        .with_size(600, 400)
        .with_title("Salary by Department")
        .save("./tests/scatter_9.svg")?;

    Ok(())
}

#[test]
fn test_scatter_10() -> Result<(), Box<dyn Error>> {
    // Create sample data with a continuous variable for color encoding
    let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let b = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
    let value = [1.5, 2.3, 3.7, 4.1, 5.9, 6.2, 7.8, 8.4, 9.1, 10.6]; // Continuous variable for color

    // Create a chart with color encoding using the continuous variable
    let chart = chart!(a, b, value)?.mark_point()?.encode((
        alt::x("a"),
        alt::y("b"),
        alt::color("value"), // This will trigger the colorbar instead of discrete legend
        alt::size("value"),  // This will trigger the colorbar instead of discrete legend
    ))?;

    chart
        .with_size(500, 300)
        .with_title("Chart with Colorbar")
        .save("./tests/scatter_10.svg")?;

    Ok(())
}

#[test]
fn test_scatter_11() -> Result<(), Box<dyn Error>> {
    // Create sample data with a categorical variable for shape encoding
    let a = [1.0, 2.0, 3.0, 4.0, 5.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let b = [10.0, 20.0, 30.0, 40.0, 50.0, 15.0, 25.0, 35.0, 45.0, 55.0];
    let category = ["A", "B", "C", "A", "B", "C", "A", "B", "C", "A"]; // Categorical variable for shape

    // Create a chart with shape encoding using the categorical variable
    chart!(a, b, category)?
        .mark_point()?
        .encode((
            alt::x("a"),
            alt::y("b"),
            //size("x"),
            alt::shape("category"), // This will trigger the shape legend
            alt::color("category"), // This will trigger the color legend
        ))?
        .with_size(500, 300)
        .with_title("Chart with Shape Legend")
        .save("./tests/scatter_11.svg")?;

    Ok(())
}

#[test]
fn test_scatter_12() -> Result<(), Box<dyn Error>> {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0];
    let b = [10.0, 20.0, 30.0, 40.0, 50.0];
    let category = ["A", "B", "A", "B", "C"];

    // Create a point chart with only x, y, and color encodings
    chart!(a, b, category)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("category")))?
        .with_size(500, 300)
        .save("./tests/scatter_12.svg")?;

    Ok(())
}

#[test]
fn test_scatter_13() -> Result<(), Box<dyn Error>> {
    // Create sample data
    let a = [1.0, 2.0, 3.0, 4.0, 5.0];
    let b = [10.0, 20.0, 30.0, 40.0, 50.0];
    let category = ["A", "B", "C", "D", "E"];

    chart!(a, b, category)?
        .mark_point()?
        .encode((alt::x("category"), alt::y("b"), alt::color("category")))?
        .with_size(600, 400)
        .with_title("Chart with Explicit Tick Values")
        .with_x_label("Catergory")
        .with_y_label("B Values")
        .save("./tests/scatter_13.svg")?;

    Ok(())
}

#[test]
fn test_scatter_14() -> Result<(), Box<dyn Error>> {
    let a = [130.0, 165.0, 156.0, 1500.0];
    let b = [18.0, 15.0, 20.0, 16.0];
    let origin = ["USA", "USA", "Europe", "Japan"];

    chart!(a, b, origin)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("origin")))?
        .save("./tests/scatter_14.svg")?;

    Ok(())
}
