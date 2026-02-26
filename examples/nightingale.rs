use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = load_dataset("nightingale")?;
    println!("{:?}", df);

    // 1. Build the chart layer. 
    let chart = Chart::build(&df)?
        .mark_bar()?
        .encode((
            x("Month"),
            y("Deaths").with_stack(true).with_normalize(false),
            color("Cause"),
        ))?;

    // 2. Final Assembly in Polar Coordinates.
    LayeredChart::new()
        .with_title("Nightingale wind rose")
        .add_layer(chart)
        .with_coord(CoordSystem::Polar)
        .save("./examples/nightingale.svg")?;

    Ok(())
}