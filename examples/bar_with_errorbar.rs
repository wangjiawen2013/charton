use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let type1 = ["a", "b", "c", "d"];
    let value = [4.9, 5.3, 5.5, 6.5];
    let value_std = [0.3, 0.39, 0.34, 0.20];

    // Create error bar chart using transform_calculate to add min/max values
    let errorbar = chart!(type1, value, value_std)?
        // Use transform_calculate to create ymin and ymax columns based on fixed std values
        .transform_calculate(
            (col("value") - col("value_std")).alias("value_min"), // ymin = y - std
            (col("value") + col("value_std")).alias("value_max"), // ymax = y + std
        )?
        .mark_errorbar()?
        .encode((alt::x("type1"), alt::y("value_min"), alt::y2("value_max")))?;
    let bar = chart!(type1, value)?
        .mark_bar()?
        .encode((alt::x("type1"), alt::y("value")))?;

    // Create a layered chart and add the errorbar chart as a layer
    errorbar
        .and(bar)
        .with_y_label("value")
        .save("docs/src/images/bar_with_errorbar.svg")?;

    Ok(())
}
