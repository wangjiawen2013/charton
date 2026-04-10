use charton::prelude::*;

#[test]
fn test_scatter_1() -> Result<(), Box<dyn std::error::Error>> {
    let a = [Some(130.0), None, Some(156.0), Some(1500.0), None];
    let b = [-0.0001, -0.002, 0.001, 0.003, 1.0];
    let c = ["USA", "USA", "Europe", "USA", "Japan"];

    chart!(a, b, c)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b")))?
        .with_size(500, 400)
        .to_svg()?;

    Ok(())
}
