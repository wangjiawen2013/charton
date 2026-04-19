use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("nightingale")?;
    println!("{:?}", ds);

    chart!(ds)?
        .mark_bar()?
        .encode((
            alt::x("Month"),
            alt::y("Deaths").with_stack("stacked").with_normalize(false),
            alt::color("Cause"),
        ))?
        .with_title("Nightingale wind rose")
        .with_coord(CoordSystem::Polar)
        .save("docs/src/images/nightingale.svg")?;

    Ok(())
}
