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

fn text_position(svg: &str, text: &str) -> (f64, f64) {
    let marker = format!(">{text}</text>");
    let line = svg
        .lines()
        .find(|line| line.contains(&marker))
        .expect("expected legend title in SVG");

    let x = line
        .split(" x=\"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .expect("expected x coordinate")
        .parse::<f64>()
        .expect("x coordinate should be numeric");
    let y = line
        .split(" y=\"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .expect("expected y coordinate")
        .parse::<f64>()
        .expect("y coordinate should be numeric");

    (x, y)
}

#[test]
fn legend_position_changes_svg_anchor() -> Result<(), Box<dyn std::error::Error>> {
    use charton::core::guide::LegendPosition;

    let build_chart = |position| {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![2.0, 4.0, 3.0, 5.0];
        let group = vec!["A", "B", "A", "B"];

        chart!(x, y, group)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y"), alt::color("group")))?
            .configure_theme(|theme| theme.with_legend_position(position))
            .to_svg()
    };

    let right = text_position(&build_chart(LegendPosition::Right)?, "group");
    let left = text_position(&build_chart(LegendPosition::Left)?, "group");
    let top = text_position(&build_chart(LegendPosition::Top)?, "group");
    let bottom = text_position(&build_chart(LegendPosition::Bottom)?, "group");

    assert!(
        left.0 < right.0,
        "left legend should be left of right legend"
    );
    assert!(top.1 < bottom.1, "top legend should be above bottom legend");

    Ok(())
}
