use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data similar to the Iowa electricity dataset
    let df = df! [
        "year" => [2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012],
        "net_generation" => [50.0, 70.0, 130.0, 160.0, 180.0, 200.0, 170.0, 140.0, 90.0, 60.0, 80.0, 90.0, 100.0, 110.0, 120.0, 130.0, 140.0, 120.0, 90.0, 70.0],
        "source" => ["Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear"],
    ]?;

    // Create an area chart
    let area_chart = Chart::build(&df)?
        .mark_area()
        .with_area_opacity(0.3)
        .with_area_stroke(Some(SingleColor::new("black")))
        .encode((x("year"), y("net_generation"), color("source")))?;

    // Create a layered chart for the area
    LayeredChart::new()
        .with_title("Iowa Electricity Generation")
        .with_x_label("Year")
        .with_y_label("Net Generation")
        .add_layer(area_chart)
        .save("./examples/area.svg")?;

    Ok(())
}
