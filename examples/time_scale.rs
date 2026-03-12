use charton::prelude::*;
use polars::prelude::*;
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

    // Convert OffsetDateTime to nanosecond timestamps (i64)
    // This is the standard internal representation for Polars Datetime series.
    let date_values: Vec<i64> = dates
        .into_iter()
        .map(|dt| dt.unix_timestamp_nanos() as i64)
        .collect();

    let values = [10.5, 25.2, 45.0, 30.8, 60.3];

    // 2. Construct the DataFrame
    // We explicitly cast the integer timestamps to a Datetime type with Nanosecond precision.
    let date_series = Series::new("date".into(), date_values)
        .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))?;
    let val_series = Series::new("value".into(), values);

    let df = DataFrame::new(vec![date_series.into(), val_series.into()])?;
    println!("DataFrame: {:?}", df);

    // 3. Build the chart
    Chart::build(&df)?.mark_point()?.encode((
        x("date"),
        y("value"),
    ))?
        .with_size(500, 400)
        .configure_theme(|t| {
            t.with_x_tick_label_angle(-45.0)
                .with_tick_label_size(12.0)
        })
        .save("docs/src/images/time_scale.svg")?;

    Ok(())
}
