use charton::prelude::*;
use std::error::Error;

#[test]
fn test_transform_calculate_1() -> Result<(), Box<dyn Error>> {
    // Create sample data with x and y values
    let type1 = ["a", "b", "c", "d"];
    let value = [5.1, 5.3, 5.7, 6.5];
    let value_std = [0.2, 0.23, 0.14, 0.25];

    // Create error bar chart using transform_calculate to add min/max values
    let errorbar_chart = chart!(type1, value, value_std)?
        // Use transform_calculate to create ymin and ymax columns based on fixed std values
        .transform_calculate(
            (col("value") - col("value_std")).alias("value_min"), // ymin = y - std
            (col("value") + col("value_std")).alias("value_max"), // ymax = y + std
        )?
        .mark_errorbar()?
        .encode((alt::x("type"), alt::y("value_min"), alt::y2("value_max")))?
        .configure_errorbar(
            |e| {
                e.with_color("blue")
                    .with_stroke_width(2.0)
                    .with_cap_length(5.0)
                    .with_center(true)
            }, // Show center point
        );
    errorbar_chart
        .with_size(500, 400)
        .with_title("Error Bar Chart with Mean and Std Dev")
        .coord_flip()
        .save("./tests/transform_calculate_1.svg")?;

    Ok(())
}
