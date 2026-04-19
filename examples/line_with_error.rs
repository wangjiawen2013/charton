use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let type1 = [1.2, 2.2, 3.0, 4.1];
    let value = [4.9, 5.3, 5.5, 6.5];
    let value_std = [0.2, 0.23, 0.14, 0.25];

    // Create error bar chart using transform_calculate to add min/max values
    let errorbar = chart!(type1, value, value_std)?
        // Use transform_calculate to create ymin and ymax columns based on fixed std values
        .transform_calculate("value_min", |row| {
            Some(row.val("value")? - row.val("value_std")?)
        })?
        .transform_calculate("value_max", |row| {
            Some(row.val("value")? + row.val("value_std")?)
        })?
        .mark_errorbar()?
        .encode((alt::x("type1"), alt::y("value_min"), alt::y2("value_max")))?;
    let line = chart!(type1, value, value_std)?
        .mark_line()?
        .encode((alt::x("type1"), alt::y("value")))?;

    // Create a layered chart
    let chart = errorbar.and(line);
    chart.save("docs/src/images/line_with_errorbar.svg")?;

    Ok(())
}
