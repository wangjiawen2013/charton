use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "Month" => ["Jan", "Jan", "Jan", "Jan", "Feb", "Feb", "Feb", "Feb", "Mar", "Mar", "Mar", "Mar"],
        //"Month" => [1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3],
        "Revenue" => [500.0, 120.1, 90.0, 140.0, 110.0, 130.0, 100.0, 120.0, 90.0, 140.0, 110.0, 130.0],
        "Region" => ["North", "South", "East", "West", "North", "South", "East", "West", "North", "South", "East", "West"],
    ]?;

    // Create a bar chart with color encoding
    let colored_bar_chart = Chart::build(&df)?
        .mark_bar()?
        .configure_bar(|b| {
            b.with_stroke("black")
                .with_stroke_width(1.0)
                .with_width(0.5)
        })
        .encode((
            x("Month"),
            y("Revenue").with_normalize(true).with_stack(true),
            color("Region"),
        ))?;

    // Create a layered chart for colored bars
    LayeredChart::new()
        .with_title("Colored Bar Chart Example")
        .add_layer(colored_bar_chart)
        .save("./examples/stacked_bar.svg")?;

    Ok(())
}
