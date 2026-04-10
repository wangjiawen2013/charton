use charton::prelude::*;
use std::error::Error;
use time::macros::datetime;

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Create source data using the 'time' crate macros
    // We define a set of timestamps spanning exactly one year.
    let dates = vec![
        datetime!(2025-01-01 00:00:00 UTC),
        datetime!(2025-04-01 00:00:00 UTC),
        datetime!(2025-07-01 00:00:00 UTC),
        datetime!(2025-10-01 00:00:00 UTC),
        datetime!(2026-01-01 00:00:00 UTC),
    ];
    let values = [10.5, 25.2, 45.0, 30.8, 60.3];

    // 3. Build the chart
    chart!(dates, values)?
        .mark_point()?
        .encode((alt::x("dates"), alt::y("values")))?
        .with_size(500, 400)
        .configure_theme(|t| t.with_x_tick_label_angle(-45.0).with_tick_label_size(12.0))
        .save("docs/src/images/time_scale.svg")?;

    Ok(())
}
