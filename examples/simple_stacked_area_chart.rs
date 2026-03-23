use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data similar to the Iowa electricity dataset
    let df = load_dataset("unemployment")?;
    println!("{}", df);
    // Create an area chart
    let area_chart = Chart::build(&df)?
        .mark_area()?
        .encode((
            x("Year"),
            y("Unemployment rate (%)").with_stack("stacked"),
            color("Country"),
        ))?;

    // Create a layered chart for the area
    area_chart
        .save("docs/src/images/simple_stacked_area.svg")?;

    Ok(())
}