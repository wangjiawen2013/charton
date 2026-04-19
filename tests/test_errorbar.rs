use charton::prelude::*;
use std::error::Error;

#[test]
fn test_errorbar_1() -> Result<(), Box<dyn Error>> {
    // Create sample data with multiple points having the same x values,
    // which allows calculating mean and standard deviation
    let x = ["a", "a", "a", "b", "b", "b", "c", "c", "c", "d", "d", "d"];
    let y = [5.1, 5.3, 5.7, 6.5, 6.9, 6.2, 4.0, 4.2, 4.4, 7.6, 8.0, 7.8];

    // Create error bar chart
    chart!(x, y)?
        .mark_errorbar()?
        .configure_errorbar(|e| {
            e.with_color("blue")
                .with_stroke_width(2.0)
                .with_cap_length(5.0)
                .with_center(true)
        })
        .encode((alt::x("x"), alt::y("y")))?
        .with_size(500, 400)
        .with_title("Error Bar Chart with Mean and Std Dev")
        .save("./tests/errorbar_1.svg")?;

    Ok(())
}

#[test]
fn test_errorbar_2() -> Result<(), Box<dyn Error>> {
    // 1. Create sample data with an extra "group" column
    // Each combination of (x, group) has multiple values for std dev calculation
    let x = ["A", "A", "A", "A", "A", "A", "B", "B", "B", "B", "B", "B"];
    let y = [
        10.0, 12.0, 11.0, 15.0, 17.0, 16.0, 8.0, 9.0, 8.5, 12.0, 14.0, 13.0,
    ];
    let group = [
        "G1", "G1", "G1", "G2", "G2", "G2", "G1", "G1", "G1", "G2", "G2", "G2",
    ];

    // 2. Build the Error Bar layer
    let errorbar_layer = chart!(x, y, group)?
        .mark_errorbar()?
        // Mapping 'group' to color triggers the dodge logic
        .encode((alt::x("x"), alt::y("y"), alt::color("group")))?;

    // 3. (Optional but recommended) Add a Bar layer to see the alignment
    let bar_layer = chart!(x, y, group)?.mark_bar()?.encode((
        alt::x("x"),
        alt::y("y").with_aggregate("mean"),
        alt::color("group"),
    ))?;

    // 4. Create the Layered Chart
    errorbar_layer
        .and(bar_layer)
        .with_size(600, 400)
        .with_title("Grouped Error Bars with Mean & Std Dev")
        .save("./tests/errorbar_2.svg")?;

    Ok(())
}
