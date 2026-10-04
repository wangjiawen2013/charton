use charton::prelude::*;
use std::error::Error;

/// Counts the marker elements inside the plot clip group, excluding legend keys.
fn plot_circles(svg: &str) -> usize {
    let start = svg.find("<g clip-path=").expect("plot group missing");
    let rest = &svg[start..];
    let end = rest.find("</g>").expect("plot group not closed");
    rest[..end].matches("<circle").count()
}

/// Missing **positional** values (x/y) are dropped, matching Vega-Lite and
/// ggplot2: there is no "NA" axis tick and no fabricated category.
#[test]
fn missing_positions_are_dropped() -> Result<(), Box<dyn Error>> {
    // x: ["A", null, "A", null, "B"], y: 1..=5.
    let x = vec![Some("A"), None, Some("A"), None, Some("B")];
    let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];

    // Scatter: only the three non-null points survive.
    chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .save("./target/test-output/null_position_scatter.svg")?;
    let scatter = std::fs::read_to_string("./target/test-output/null_position_scatter.svg")?;
    assert!(!scatter.contains(">NA<"), "scatter invented an NA tick");
    assert_eq!(
        plot_circles(&scatter),
        3,
        "scatter must drop the two null points"
    );

    // Boxplot: only the A and B boxes exist.
    chart!(&x, &y)?
        .mark_boxplot()?
        .encode((alt::x("x"), alt::y("y")))?
        .save("./target/test-output/null_position_boxplot.svg")?;
    let boxplot = std::fs::read_to_string("./target/test-output/null_position_boxplot.svg")?;
    assert!(!boxplot.contains(">NA<"), "boxplot invented an NA tick");
    // Background + clip rect + one box per non-null category (A, B).
    assert_eq!(
        boxplot.matches("<rect").count(),
        4,
        "boxplot drew an NA box"
    );

    // Bar: only the A and B bars exist.
    chart!(&x, &y)?
        .mark_bar()?
        .encode((alt::x("x"), alt::y("y").with_aggregate("mean")))?
        .save("./target/test-output/null_position_bar.svg")?;
    let bar = std::fs::read_to_string("./target/test-output/null_position_bar.svg")?;
    assert!(!bar.contains(">NA<"), "bar invented an NA tick");
    assert_eq!(bar.matches(" Z\"").count(), 2, "bar drew an NA bar");

    Ok(())
}

/// A missing **non-positional** value (colour) keeps the observation, mapped to
/// the reserved grey "NA" level and listed in the legend.
#[test]
fn missing_colour_keeps_the_observation() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![1.0, 2.0, 3.0, 4.0];
    // g: ["a", null, "b", null]
    let g = vec![Some("a"), None, Some("b"), None];

    chart!(&x, &y, &g)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y"), alt::color("g")))?
        .save("./target/test-output/null_colour_scatter.svg")?;

    let svg = std::fs::read_to_string("./target/test-output/null_colour_scatter.svg")?;
    assert!(svg.contains(">NA<"), "legend must list the NA level");
    assert_eq!(
        plot_circles(&svg),
        4,
        "a missing colour must not drop a valid observation"
    );
    assert!(
        svg.contains("rgba(128,128,128,1.000)"),
        "the NA level must be painted grey"
    );

    Ok(())
}

/// A layer that does not map colour must keep its own fallback colour, even
/// when another layer in the same chart maps colour (the resolver must not
/// confuse "no colour channel" with "missing colour").
#[test]
fn layer_without_colour_keeps_its_fallback() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0];
    let y1 = vec![1.0, 2.0, 3.0];
    let y2 = vec![3.0, 2.0, 1.0];
    let g = vec!["a", "b", "a"];

    let coloured =
        chart!(&x, &y1, &g)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y1"), alt::color("g")))?;
    let plain = chart!(&x, &y2)?
        .mark_point()?
        .configure_point(|p| p.with_color("red"))
        .encode((alt::x("x"), alt::y("y2")))?;

    coloured
        .and(plain)
        .save("./target/test-output/null_layer_without_colour.svg")?;

    let svg = std::fs::read_to_string("./target/test-output/null_layer_without_colour.svg")?;
    assert!(
        svg.contains("rgba(255,0,0,1.000)"),
        "a layer without a colour mapping must keep its red fallback"
    );

    Ok(())
}

/// A missing value on a **continuous** colour scale is painted the same grey
/// `na.value`, and the observation is still drawn.
#[test]
fn missing_continuous_colour_is_grey() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![1.0, 2.0, 3.0, 4.0];
    // c: [0, null, 2, 3] -> a continuous (Linear) colour scale.
    let c = vec![0.0, f64::NAN, 2.0, 3.0];

    chart!(&x, &y, &c)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y"), alt::color("c")))?
        .save("./target/test-output/null_continuous_colour.svg")?;

    let svg = std::fs::read_to_string("./target/test-output/null_continuous_colour.svg")?;
    assert_eq!(plot_circles(&svg), 4, "must keep all four observations");
    assert!(
        svg.contains("rgba(128,128,128,1.000)"),
        "a continuous NA colour must be painted grey"
    );

    Ok(())
}

/// A missing position on a line/area breaks the path (a visible gap), rather
/// than connecting across it — the failed-sensor-interval case.
#[test]
fn missing_position_breaks_line_and_area() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];

    // --- Line ---
    let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    chart!(&x, &y)?
        .mark_line()?
        .encode((alt::x("x"), alt::y("y")))?
        .save("./target/test-output/null_line_full.svg")?;
    let line_full = std::fs::read_to_string("./target/test-output/null_line_full.svg")?;

    let y = vec![1.0, 2.0, f64::NAN, 4.0, 5.0];
    chart!(&x, &y)?
        .mark_line()?
        .encode((alt::x("x"), alt::y("y")))?
        .save("./target/test-output/null_line_gap.svg")?;
    let line_gap = std::fs::read_to_string("./target/test-output/null_line_gap.svg")?;

    assert_eq!(
        line_gap.matches("<path").count(),
        line_full.matches("<path").count() + 1,
        "a null y must split the line into two paths"
    );

    // --- Area ---
    let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    chart!(&x, &y)?
        .mark_area()?
        .encode((alt::x("x"), alt::y("y")))?
        .save("./target/test-output/null_area_full.svg")?;
    let area_full = std::fs::read_to_string("./target/test-output/null_area_full.svg")?;

    let y = vec![1.0, 2.0, f64::NAN, 4.0, 5.0];
    chart!(&x, &y)?
        .mark_area()?
        .encode((alt::x("x"), alt::y("y")))?
        .save("./target/test-output/null_area_gap.svg")?;
    let area_gap = std::fs::read_to_string("./target/test-output/null_area_gap.svg")?;

    assert!(
        area_gap.matches("<path").count() > area_full.matches("<path").count(),
        "a null y must split the area into more paths"
    );

    Ok(())
}
