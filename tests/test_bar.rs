use charton::prelude::*;
use std::error::Error;
use std::fs;

#[test]
fn test_bar_1() -> Result<(), Box<dyn Error>> {
    let month = vec![
        "Jan", "Jan", "Jan", "Jan", "Feb", "Feb", "Feb", "Feb", "Mar", "Mar", "Mar", "Mar",
    ];
    let revenue = vec![
        500.0, 120.1, 90.0, 140.0, 110.0, 130.0, 100.0, 120.0, 90.0, 140.0, 110.0, 130.0,
    ];
    let region = vec![
        "North", "South", "East", "West", "North", "South", "East", "West", "North", "South",
        "East", "West",
    ];

    // Create a bar chart with color encoding
    let colored_bar_chart = chart!(month, revenue, region)?
        .mark_bar()?
        .configure_bar(|b| {
            b.with_stroke("black")
                .with_stroke_width(1.0)
                .with_width(0.5)
        })
        .encode((
            alt::x("month"),
            alt::y("revenue").with_normalize(true).with_stack("stacked"),
            alt::color("region"),
        ))?;

    colored_bar_chart
        .with_size(600, 400)
        .with_title("Colored Bar Chart Example")
        .coord_flip()
        .save("./target/test-output/bar_1.svg")?;

    Ok(())
}

/// A non-faceted chart does not need the pre-statistic snapshot. Releasing it
/// drops the raw rows and leaves only the aggregated result.
#[test]
fn without_source_data_switches_to_post_statistic_data() {
    use charton::core::layer::Layer;

    let ds = Dataset::new()
        .with_column("x", vec!["a", "a", "b"])
        .unwrap()
        .with_column("y", vec![1.0, 2.0, 3.0])
        .unwrap();

    let chart = Chart::build(ds)
        .unwrap()
        .mark_bar()
        .unwrap()
        .encode((alt::x("x"), alt::y("y")))
        .unwrap();

    // Before release the layer exposes the rows kept for per-panel statistics.
    assert_eq!(chart.get_dataset().height(), 3);

    // After release it exposes only the aggregated result (two bars).
    let released = chart.without_source_data();
    assert_eq!(released.get_dataset().height(), 2);
}

/// A `y2` bound turns the bar into a *floating* bar: it spans the two values in
/// either order instead of growing from the baseline, and colour stays an
/// attribute (no dodging lanes).
#[test]
fn floating_bar_spans_y_to_y2() -> Result<(), Box<dyn Error>> {
    let step = ["A", "B", "C", "D"];
    let base = [10.0, 25.0, 20.0, 40.0];
    let top = [25.0, 20.0, 40.0, 35.0];
    let kind = ["up", "down", "up", "down"];

    chart!(step, base, top, kind)?
        .mark_bar()?
        .configure_bar(|bar| bar.with_width(0.6))
        .encode((
            alt::x("step"),
            alt::y("base"),
            alt::y2("top"),
            alt::color("kind"),
        ))?
        .save("./target/test-output/bar_floating.svg")?;

    Ok(())
}

/// A floating bar (`y2`) is an interval, not a magnitude: it must NOT force a
/// zero baseline, or the interval would be squashed against the axis.
#[test]
fn floating_bar_does_not_force_zero() -> Result<(), Box<dyn Error>> {
    let step = ["A", "B", "C"];
    let base = [100.0, 108.0, 104.0];
    let top = [112.0, 110.0, 118.0];

    chart!(step, base, top)?
        .mark_bar()?
        .encode((alt::x("step"), alt::y("base"), alt::y2("top")))?
        .save("./target/test-output/bar_floating_zoom.svg")?;

    // The y axis should be on the interval (100..118), not include zero.
    let svg = fs::read_to_string("./target/test-output/bar_floating_zoom.svg")?;
    assert!(
        !svg.contains(">0</text>"),
        "floating bar should not force zero"
    );
    assert!(svg.contains(">100</text>"), "expected an interval tick");
    Ok(())
}
