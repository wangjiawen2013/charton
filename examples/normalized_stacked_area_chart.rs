use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("unemployment")?;
    println!("{:?}", ds.tail(5));
    let area_chart = chart!(ds)?.mark_area()?.encode((
        alt::x("Year"),
        alt::y("Unemployment rate (%)").with_stack("normalize"),
        alt::color("Country"),
    ))?;

    area_chart.save("docs/src/images/normalized_stacked_area.svg")?;

    Ok(())
}
