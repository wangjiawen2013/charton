use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = load_dataset("unemployment")?;
    println!("{}", df);
    let area_chart = Chart::build(&df)?.mark_area()?.encode((
        x("Year"),
        y("Unemployment rate (%)").with_stack("center"),
        color("Country"),
    ))?;

    area_chart.save("docs/src/images/steamgraph.svg")?;

    Ok(())
}
