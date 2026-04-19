use charton::prelude::*;
use std::error::Error;

#[test]
fn test_rect_1() -> Result<(), Box<dyn Error>> {
    // Sample data for heatmap
    let a = ["A", "B", "C", "A", "B", "C", "A", "B", "C"];
    let b = ["X", "X", "X", "Y", "Y", "Y", "Z", "Z", "Z"];
    let value = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

    // Create heatmap chart
    chart!(a, b, value)?
        .mark_rect()?
        .encode((alt::x("a"), alt::y("b"), alt::color("value")))?
        .with_size(500, 400)
        .save("./tests/rect_1.svg")?;

    Ok(())
}

#[test]
fn test_rect_2() -> Result<(), Box<dyn Error>> {
    let a = [1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4];
    let b = [1, 2, 1, 2, 3, 1, 2, 3, 1, 2, 3];
    let value = [1.0, 2.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];

    // Create heatmap chart
    chart!(a, b, value)?
        .mark_rect()?
        .encode((alt::x("a"), alt::y("b"), alt::color("value")))?
        .with_size(500, 400)
        .coord_flip()
        .save("./tests/rect_2.svg")?;

    Ok(())
}

#[test]
fn test_rect_3() -> Result<(), Box<dyn Error>> {
    // Sample data for heatmap with continuous variables
    let x = [1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.8, 2.05, 2.2, 2.5, 2.6, 2.7];
    let y = [1.2, 1.3, 1.4, 1.5, 1.8, 1.83, 2.0, 1.9, 2.2, 2.3, 2.4, 2.5];
    let value = [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
    ];
    // Create heatmap chart
    chart!(x, y, value)?
        .mark_rect()?
        .encode((alt::x("x"), alt::y("y"), alt::color("value")))?
        .with_size(500, 400)
        .configure_theme(|t| t.with_color_map(ColorMap::GnBu))
        .save("./tests/rect_3.svg")?;

    Ok(())
}
