use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data frame for pie chart
    let df = df![
        "category" => ["A", "B", "C", "E", "D", "E"],
        "value" => [25.0, 30.0, 15.0, 30.0, 20.0, 10.0]
    ]?;

    // Create pie chart
    let pie_chart = Chart::build(&df)?
        .mark_arc() // Use arc mark for pie charts
        .encode((
            theta("value"),    // theta encoding for pie slices
            color("category"), // color encoding for different segments
        ))?
        .with_inner_radius_ratio(0.5); // Creates a donut chart

    // Create a layered chart and add the pie chart as a layer
    LayeredChart::new()
        .add_layer(pie_chart)
        .save("./examples/donut.svg")?;

    Ok(())
}
