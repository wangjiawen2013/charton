use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // The historical data
    let global = df!(
        "temperature" => &[10, 15, 20, 25, 30, 35, 40, 45, 50, 55],
        "zero" => &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        "growth" => &[2, 5, 12, 25, 40, 60, 85, 95, 98, 99]
    )?;
    
    // The sample data
    let sample = df!(
        "temperature" => &[11, 18, 25, 30, 37],
        "growth" => &[3, 8, 20, 41, 65]
    )?;

    let points = Chart::build(&sample)?
        .mark_point()?.configure_point(|p| p.with_stroke("white").with_size(6.0).with_stroke_width(1.0))
        .encode((x("temperature"), y("growth"), color("growth")))?;

    let rules = Chart::build(&global)?
        .mark_rule()?.configure_rule(|r| r.with_opacity(0.8).with_stroke_width(32.0))
        .encode((x("temperature"), y("zero"), y2("growth"), color("growth")))?;

    rules.and(points)
        .with_y_label("growth")
        .with_x_expand(Expansion { mult: (0.1, 0.1), add: (0.0, 0.0) })
        .with_y_expand(Expansion { mult: (0.0, 0.1), add: (0.0, 0.0) })
        .save("industry_charton.svg")?;

    Ok(())
}