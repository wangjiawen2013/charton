use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("unemployment")?;
    println!("{}", ds);
    let area_chart = Chart::build(&ds)?.mark_area()?.encode((
        alt::x("Year"),
        alt::y("Unemployment rate (%)").with_stack("center"),
        alt::color("Country"),
    ))?;

    area_chart.save("docs/src/images/steamgraph.svg")?;

    Ok(())
}
