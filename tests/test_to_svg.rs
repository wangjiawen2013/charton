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

/// Reads a numeric attribute from an SVG element line.
fn attr(line: &str, name: &str) -> f64 {
    let needle = format!(" {name}=\"");
    line.split(&needle)
        .nth(1)
        .and_then(|value| value.split('"').next())
        .unwrap_or_else(|| panic!("expected attribute {name} in: {line}"))
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("attribute {name} should be numeric in: {line}"))
}

/// The plot panel rectangle, in the same pixel space as everything else.
fn panel_rect(svg: &str) -> (f64, f64, f64, f64) {
    let line = svg
        .lines()
        .find(|line| line.contains("plot-clip-area-0"))
        .expect("expected the plot panel clip rect");
    (
        attr(line, "x"),
        attr(line, "y"),
        attr(line, "width"),
        attr(line, "height"),
    )
}

/// The gradient bar rectangle, in the same pixel space as everything else.
fn gradient_rect(svg: &str) -> (f64, f64, f64, f64) {
    let line = svg
        .lines()
        .find(|line| line.contains("fill=\"url(#grad"))
        .expect("expected a gradient bar rect");
    (
        attr(line, "x"),
        attr(line, "y"),
        attr(line, "width"),
        attr(line, "height"),
    )
}

fn gradient_bar_chart(
    width: u32,
    height: u32,
    position: charton::core::guide::LegendPosition,
) -> Result<String, Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..40).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| (v * 0.35).sin() * 10.0).collect();
    let value: Vec<f64> = (0..40).map(|i| (i as f64) * 1.7 - 20.0).collect();

    let svg = chart!(x, y, value)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y"), alt::color("value")))?
        .with_size(width, height)
        .configure_theme(|theme| theme.with_legend_position(position))
        .to_svg()?;

    Ok(svg)
}

/// A gradient bar must be drawn at the size the layout reserved for it, which in
/// turn is derived from the panel it sits next to. It used to be drawn at a fixed
/// 150px length, so on a short canvas it spilled over the x-axis, and on a tall
/// one it looked lost.
#[test]
fn colorbar_is_drawn_at_the_measured_size() -> Result<(), Box<dyn std::error::Error>> {
    use charton::core::guide::LegendPosition;

    for (width, height) in [(720, 260), (720, 480), (400, 300), (720, 1000)] {
        // --- vertical bar: length scales with the panel height, capped at 200 ---
        let svg = gradient_bar_chart(width, height, LegendPosition::Right)?;
        let (_, panel_y, _, panel_h) = panel_rect(&svg);
        let (_, bar_y, _, bar_h) = gradient_rect(&svg);

        let expected = (panel_h * 0.7).min(200.0);
        assert!(
            (bar_h - expected).abs() < 0.5,
            "{width}x{height}: vertical bar is {bar_h} long, expected {expected}"
        );
        assert!(
            bar_y + bar_h <= panel_y + panel_h,
            "{width}x{height}: bar ends at {}, past the panel bottom {}",
            bar_y + bar_h,
            panel_y + panel_h
        );

        // --- horizontal bar: length follows the panel width, clamped to 150..300 ---
        let svg = gradient_bar_chart(width, height, LegendPosition::Bottom)?;
        let (panel_x, _, panel_w, _) = panel_rect(&svg);
        let (bar_x, _, bar_w, _) = gradient_rect(&svg);

        let expected = panel_w.clamp(150.0, 300.0);
        assert!(
            (bar_w - expected).abs() < 0.5,
            "{width}x{height}: horizontal bar is {bar_w} long, expected {expected}"
        );
        assert!(
            bar_x + bar_w <= panel_x + panel_w,
            "{width}x{height}: bar ends at {}, past the panel right edge {}",
            bar_x + bar_w,
            panel_x + panel_w
        );
    }

    Ok(())
}

/// A legend far too large to fit must be clipped to the region outside the plot
/// panel. It used to be drawn straight over the data on the left and top edges,
/// and off the canvas on the right and bottom edges.
#[test]
fn an_oversized_legend_is_clipped_to_the_band_outside_the_panel()
-> Result<(), Box<dyn std::error::Error>> {
    use charton::core::guide::LegendPosition;

    for position in [
        LegendPosition::Right,
        LegendPosition::Left,
        LegendPosition::Top,
        LegendPosition::Bottom,
    ] {
        // 300 categories on a 720x480 canvas: nothing could ever make this fit.
        let n = 1200;
        let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| (v * 0.35).sin() * 10.0).collect();
        let cat: Vec<String> = (0..n).map(|i| format!("Category {:03}", i % 300)).collect();

        let svg = chart!(x, y, cat)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y"), alt::color("cat")))?
            .with_size(720, 480)
            .configure_theme(|theme| theme.with_legend_position(position))
            .to_svg()?;

        let (px, py, pw, ph) = panel_rect(&svg);

        // The legend is drawn last, so the highest clip id belongs to its band.
        let band = svg
            .lines()
            .filter(|line| line.contains("<clipPath id="))
            .next_back()
            .map(|line| {
                (
                    attr(line, "x"),
                    attr(line, "y"),
                    attr(line, "width"),
                    attr(line, "height"),
                )
            })
            .expect("expected a clip path for the legend band");

        let (bx, by, bw, bh) = band;
        let overlap_x = (bx + bw).min(px + pw) - bx.max(px);
        let overlap_y = (by + bh).min(py + ph) - by.max(py);

        assert!(
            overlap_x.max(0.0) * overlap_y.max(0.0) == 0.0,
            "{position:?}: legend band ({bx},{by},{bw}x{bh}) covers the panel \
             ({px},{py},{pw}x{ph})"
        );
    }

    Ok(())
}
