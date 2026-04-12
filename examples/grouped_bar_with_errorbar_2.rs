use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let type1 = ["a", "a", "a", "a", "b", "b", "b", "b", "c", "c", "c", "c"];
    let value = [4.1, 5.3, 5.5, 6.5, 4.2, 5.1, 5.7, 5.5, 4.3, 5.5, 5.1, 6.8];
    let value_std = [
        0.22, 0.26, 0.14, 0.23, 0.2, 0.23, 0.12, 0.25, 0.21, 0.20, 0.16, 0.25,
    ];
    let group = ["E", "F", "G", "H", "E", "F", "G", "H", "E", "F", "G", "H"];

    // Create error bar chart using transform_calculate to add min/max values
    let errorbar = chart!(type1, value, value_std, group)?
        // Use transform_calculate to create ymin and ymax columns based on fixed std values
        .transform_calculate("value_min", |row| {
            Some(row.val("value")? - row.val("value_std")?)
        })?
        .transform_calculate("value_max", |row| {
            Some(row.val("value")? + row.val("value_std")?)
        })?
        .mark_errorbar()?
        .encode((
            alt::x("type1"),
            alt::y("value_min"),
            alt::y2("value_max"),
            alt::color("group"),
        ))?;

    // Create a bar chart
    let bar = chart!(type1, value, value_std, group)?
        .mark_bar()?
        .encode((alt::x("type1"), alt::y("value"), alt::color("group")))?;

    // Create a layered chart
    errorbar
        .and(bar)
        .save("docs/src/images/grouped_bar_with_errorbar_2.svg")?;

    Ok(())
}
