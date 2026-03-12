use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "Month" => ["Jan", "Jan", "Jan", "Jan", "Feb", "Feb", "Feb", "Feb", "Mar", "Mar", "Mar", "Mar"],
        "Revenue" => [500.0, 120.1, 90.0, 140.0, 110.0, 130.0, 100.0, 120.0, 90.0, 140.0, 110.0, 130.0],
        "Region" => ["North", "South", "East", "West", "North", "South", "East", "West", "North", "South", "East", "West"],
    ]?;

    // Create a bar chart with color encoding
    let colored_bar_chart = Chart::build(&df)?
        .mark_bar()?
        .configure_bar(|b| {
            b.with_stroke(SingleColor::new("black"))
                .with_stroke_width(1.0)
                .with_width(0.5)
        })
        .encode((
            x("Month"),
            y("Revenue").with_normalize(true).with_stack(false),
            color("Region"),
        ))?;

    colored_bar_chart
        .with_title("Colored Bar Chart Example")
        .save("docs/src/images/grouped_bar.svg")?;

    Ok(())
}
