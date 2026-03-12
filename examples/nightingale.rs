use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = load_dataset("nightingale")?;
    println!("{:?}", df);

    Chart::build(&df)?
        .mark_bar()?
        .encode((
            x("Month"),
            y("Deaths").with_stack(true).with_normalize(false),
            color("Cause"),
        ))?
        .with_title("Nightingale wind rose")
        .with_coord(CoordSystem::Polar)
        .save("docs/src/images/nightingale.svg")?;

    Ok(())
}
