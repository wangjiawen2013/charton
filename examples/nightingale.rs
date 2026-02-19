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