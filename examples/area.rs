use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data similar to the Iowa electricity dataset
    let depth = vec![
        2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2003, 2004, 2005, 2006, 2007,
        2008, 2009, 2010, 2011, 2012,
    ];
    let net_generation = vec![
        50.0, 70.0, 130.0, 160.0, 180.0, 200.0, 170.0, 140.0, 90.0, 60.0, 80.0, 90.0, 100.0, 110.0,
        120.0, 130.0, 140.0, 120.0, 90.0, 70.0,
    ];
    let source = vec![
        "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Nuclear",
        "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear",
        "Nuclear",
    ];

    // Create an area chart
    let area_chart = chart!(depth, net_generation, source)?
        .mark_area()?
        .configure_area(|a| a.with_opacity(0.3).with_stroke("black"))
        .encode((
            alt::x("depth"),
            alt::y("net_generation").with_stack("center"),
            alt::color("source"),
        ))?;

    // Create a layered chart for the area
    area_chart
        .with_title("Electricity Generation")
        .with_x_label("Depth")
        .with_y_label("Net Generation")
        .save("docs/src/images/area.svg")?;

    Ok(())
}
