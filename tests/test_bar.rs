use charton::prelude::*;
use std::error::Error;

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
        .save("./tests/bar_1.svg")?;

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
