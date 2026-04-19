use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Sample data for heatmap
    let x = ["A", "B", "C", "A", "B", "C", "A", "B", "C"];
    let y = ["X", "X", "X", "Y", "Y", "Y", "Z", "Z", "Z"];
    let value = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

    // Create heatmap chart
    chart!(x, y, value)?
        .mark_rect()?
        .encode((alt::x("x"), alt::y("y"), alt::color("value")))?
        .save("docs/src/images/heatmap.svg")?;

    Ok(())
}
