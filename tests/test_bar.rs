use charton::prelude::*;
use std::error::Error;

#[test]
fn test_bar_1() -> Result<(), Box<dyn Error>> {
    let month = vec![
        "Jan", "Jan", "Jan", "Jan", "Feb", "Feb", "Feb", "Feb", "Mar", "Mar", "Mar", "Mar",
    ];
    let revenue = vec![
        500.0, 120.1, 90.0, 140.0, 110.0, 130.0, 100.0, 120.0, 90.0, 140.0, 110.0, 130.0,
    ];
    let region = vec![
        "North", "South", "East", "West", "North", "South", "East", "West", "North", "South",
        "East", "West",
    ];

    // Create a bar chart with color encoding
    let colored_bar_chart = chart!(month, revenue, region)?
        .mark_bar()?
        .configure_bar(|b| {
            b.with_stroke("black")
                .with_stroke_width(1.0)
                .with_width(0.5)
        })
        .encode((
            alt::x("month"),
            alt::y("revenue").with_normalize(true).with_stack("stacked"),
            alt::color("region"),
        ))?;

    colored_bar_chart
        .with_size(600, 400)
        .with_title("Colored Bar Chart Example")
        .coord_flip()
        .save("./tests/bar_1.svg")?;

    Ok(())
}
