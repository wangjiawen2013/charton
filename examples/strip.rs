use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("iris")?;

    let chart = Chart::build(&ds)?.mark_tick()?.encode((
        alt::x("sepal_width"),
        alt::y("species"),
        alt::color("species"),
    ))?;

    chart.save("docs/src/images/strip.svg")?;

    Ok(())
}
