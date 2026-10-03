use charton::prelude::*;
use std::error::Error;

/// A path connects rows in order and does **not** close the loop. Three points
/// therefore produce a two-segment polyline, not a triangle.
#[test]
fn test_path_is_open() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 3.0, 1.0];
    let g = vec!["line", "line", "line"];

    chart!(x, y, g)?
        .mark_path()?
        .configure_path(|m| m.with_stroke_width(2.0))
        .encode((alt::x("x"), alt::y("y"), alt::path_group("g")))?
        .save("./target/test-output/path_1.svg")?;

    Ok(())
}

/// A polygon closes the same three points, so the outline gains the closing
/// edge. Same geometry, one flag apart.
#[test]
fn test_polygon_is_closed() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 3.0, 1.0];
    let g = vec!["tri", "tri", "tri"];

    chart!(x, y, g)?
        .mark_polygon()?
        .encode((alt::x("x"), alt::y("y"), alt::path_group("g")))?
        .save("./target/test-output/path_2.svg")?;

    Ok(())
}
