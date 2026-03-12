use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data frame for donut chart
    let df = df![
        "category" => ["A", "B", "C", "E", "D", "E"],
        "value" => [25.0, 30.0, 15.0, 30.0, 20.0, 10.0]
    ]?;

    // Create donut chart
    let donut = Chart::build(&df)?.mark_bar()?.encode((
        x(""),             // x encoding for donut chart (empty string for donut chart)
        y("value"),        // theta encoding for donut slices
        color("category"), // color encoding for different segments
    ))?
        .with_coord(CoordSystem::Polar)
        .with_inner_radius(0.5);
    
    donut.save("docs/src/images/donut.svg")?;

    Ok(())
}
