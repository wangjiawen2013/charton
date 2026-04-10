use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data frame for donut chart
    let category = ["A", "B", "C", "E", "D", "E"];
    let value = [25.0, 30.0, 15.0, 30.0, 20.0, 10.0];

    // Create donut chart
    let donut = chart!(category, value)?
        .mark_bar()?
        .encode((
            alt::x(""),             // x encoding for donut chart (empty string for donut chart)
            alt::y("value"),        // theta encoding for donut slices
            alt::color("category"), // color encoding for different segments
        ))?
        .with_coord(CoordSystem::Polar)
        .with_inner_radius(0.5);

    donut.save("docs/src/images/donut.svg")?;

    Ok(())
}
