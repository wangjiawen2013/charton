//! Scale smoke tests: log/temporal scales and flipped (swapped) axes.
//!
//! Were `examples/log_scale.rs`, `examples/time_scale.rs` and
//! `examples/swapped_axes.rs`. Output goes to `target/test-output/`.

use charton::prelude::*;
use std::error::Error;
use time::macros::datetime;

#[test]
fn log_scale() -> Result<(), Box<dyn Error>> {
    let country = ["A", "B", "C", "D", "E"];
    let gdp = [1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0];
    let population = [100000.0, 500000.0, 2000000.0, 10000000.0, 50000000.0];

    chart!(country, gdp, population)?
        .mark_point()?
        .encode((alt::x("population"), alt::y("gdp").with_scale(Scale::Log)))?
        .with_size(500, 400)
        .configure_theme(|t| t.with_x_tick_label_angle(-45.0))
        .coord_flip()
        .save("./target/test-output/log_scale.svg")?;

    Ok(())
}

#[test]
fn time_scale() -> Result<(), Box<dyn Error>> {
    let dates = vec![
        datetime!(2025-01-01 00:00:00 UTC),
        datetime!(2025-04-01 00:00:00 UTC),
        datetime!(2025-07-01 00:00:00 UTC),
        datetime!(2025-10-01 00:00:00 UTC),
        datetime!(2026-01-01 00:00:00 UTC),
    ];
    let values = [10.5, 25.2, 45.0, 30.8, 60.3];

    chart!(dates, values)?
        .mark_point()?
        .encode((alt::x("dates"), alt::y("values")))?
        .with_size(500, 400)
        .configure_theme(|t| t.with_x_tick_label_angle(-45.0).with_tick_label_size(12.0))
        .save("./target/test-output/time_scale.svg")?;

    Ok(())
}

#[test]
fn swapped_axes() -> Result<(), Box<dyn Error>> {
    let month = vec![
        "Jan", "Jan", "Jan", "Jan", "Feb", "Feb", "Feb", "Feb", "Mar", "Mar", "Mar", "Mar",
    ];
    let revenue = vec![
        100.0, -120.1, 90.0, -140.0, 110.0, 130.0, -100.0, 120.0, 90.0, 140.0, -110.0, -130.0,
    ];
    let region = vec![
        "North", "South", "East", "West", "North", "South", "East", "West", "North", "South",
        "East", "West",
    ];

    chart!(&month, &revenue, &region)?
        .mark_bar()?
        .configure_bar(|b| {
            b.with_stroke("black")
                .with_stroke_width(1.0)
                .with_width(0.5)
        })
        .encode((
            alt::x("month"),
            alt::y("revenue").with_stack("none"),
            alt::color("region"),
        ))?
        .coord_flip()
        .save("./target/test-output/swapped_axes.svg")?;

    Ok(())
}
