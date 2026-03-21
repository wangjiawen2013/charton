use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = load_dataset("iris")?;

    let chart = Chart::build(&df)?.mark_tick()?.encode((
        x("sepal_width"),
        y("species"),
        color("species"),
    ))?;

    chart.save("docs/src/images/strip.svg")?;

    Ok(())
}
