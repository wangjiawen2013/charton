use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = df! [
        "Month" => ["Jan", "Jan", "Jan", "Jan", "Feb", "Feb", "Feb", "Feb", "Mar", "Mar", "Mar", "Mar"],
        "Revenue" => [100.0, -120.1, 90.0, -140.0, 110.0, 130.0, -100.0, 120.0, 90.0, 140.0, -110.0, -130.0],
        "Region" => ["North", "South", "East", "West", "North", "South", "East", "West", "North", "South", "East", "West"],
    ]?;

    // Create a bar chart with color encoding
    let colored_bar_chart = Chart::build(&df)?
        .mark_bar()
        .with_bar_stroke(Some(SingleColor::new("black")))
        .with_bar_stroke_width(1.0)
        .with_bar_width(0.5)
        .encode((x("Month"), y("Revenue").with_stack(false), color("Region")))?
        .swap_axes();

    // Create a layered chart for colored bars
    LayeredChart::new()
        .add_layer(colored_bar_chart)
        .save("./examples/swapped_axes.svg")?;

    Ok(())
}
