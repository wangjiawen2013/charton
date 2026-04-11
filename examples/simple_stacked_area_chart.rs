use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data similar to the Iowa electricity dataset
    let ds = load_dataset("unemployment")?;
    println!("{:?}", ds);
    // Create an area chart
    let area_chart = chart!(ds)?.mark_area()?.encode((
        alt::x("Year"),
        alt::y("Unemployment rate (%)").with_stack("stacked"),
        alt::color("Country"),
    ))?;

    // Create a layered chart for the area
    area_chart.save("docs/src/images/simple_stacked_area.svg")?;

    Ok(())
}
