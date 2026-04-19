use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Create a DataFrame with high-variance values to emphasize radial differences.
    // We use 6 categories ("A" through "F") to fill the polar coordinate space evenly.
    let type1 = ["A", "B", "C", "D", "E", "F"];
    // Data values range from 2.0 to 12.0 to create a clear "petal" effect
    let value = [3.0, 11.5, 4.2, 9.8, 2.5, 7.0];

    // 2. Build the bar chart
    // In a Polar Coordinate system, x-axis maps to the Angle (theta)
    // and y-axis maps to the Radius (r).
    chart!(type1, value)?
        .mark_bar()?
        .encode((
            alt::x("type1"),     // Each category represents a slice of the circle
            alt::y("value"),     // The height of the bar becomes the radius of the slice
            alt::color("type1"), // Distinct colors for each "petal"
        ))?
        .with_y_label("Intensity")
        // CoordSystem::Polar transforms the rectangular bar chart into a Rose Chart
        .with_coord(CoordSystem::Polar)
        .save("docs/src/images/rose.svg")?;

    Ok(())
}
