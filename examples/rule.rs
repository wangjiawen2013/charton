use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data with x, y, y2, and color columns
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [2.0, 3.0, 1.0, 4.0, 2.0];
    let y2 = [4.0, 5.0, 3.0, 6.0, 4.0];
    let color = ["A", "B", "A", "B", "A"];

    // Create a chart with rule marks using the consistent API style
    let chart = chart!(x, y, y2, color)?
        .mark_rule()?
        .encode((alt::x("x"), alt::y("y"), alt::y2("y2"), alt::color("color")))?
        .with_title("Rule Chart with Y and Y2")
        .with_x_label("X Values")
        .with_y_label("Y Values");

    // Save to SVG file
    chart.save("docs/src/images/rule.svg")?;

    Ok(())
}
