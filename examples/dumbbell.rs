//! Dumbbell (connected dot) — two values per category, joined by a segment.
//!
//! A dumbbell is a recipe, not a mark. `mark_rule` draws the segment between the
//! two values and a `mark_point` layer marks each end; all layers share the
//! category and value scales, so they line up automatically. `coord_flip`
//! stands the categories up so the segments run horizontally.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Unemployment rate (%), 2011 vs 2021, for seven countries.
    let country = [
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
    ];
    let y2011 = [6.17, 15.35, 6.68, 6.38, 15.70, 10.36, 8.17];
    let y2021 = [4.74, 6.63, 3.96, 3.50, 8.27, 3.72, 4.42];

    // 1. The connector: one rule per country, from the 2011 value to the 2021
    //    value. The rule geometry reads `y` and `y2` from one wide row.
    let connector = chart!(country, y2011, y2021)?
        .mark_rule()?
        .configure_rule(|rule| rule.with_color("#b9bfc6").with_stroke_width(2.0))
        .encode((alt::x("country"), alt::y("y2011"), alt::y2("y2021")))?;

    // 2. The two ends: a tidy (long) table, one row per country and period, so
    //    the period can drive the colour and produce a legend.
    let countries_long: Vec<&str> = country.iter().chain(country.iter()).copied().collect();
    let periods: Vec<&str> = std::iter::repeat("2011")
        .take(7)
        .chain(std::iter::repeat("2021").take(7))
        .collect();
    let values: Vec<f64> = y2011.iter().chain(y2021.iter()).copied().collect();

    let ends = Dataset::new()
        .with_column("country", countries_long)?
        .with_column("period", periods)?
        .with_column("value", values)?;

    connector
        .and(
            chart!(&ends)?
                .mark_point()?
                // `with_dodge(false)` keeps both ends on the shared segment: a
                // colour channel is otherwise a *lane*, which would push the
                // two periods apart.
                .configure_point(|point| point.with_size(9.0).with_dodge(false))
                .encode((alt::x("country"), alt::y("value"), alt::color("period")))?,
        )
        .coord_flip()
        .with_title("Unemployment rate, 2011 vs 2021")
        .with_x_label("")
        .with_y_label("Unemployment rate (%)")
        .with_color_label("Year")
        .save("docs/src/images/dumbbell.svg")?;

    println!("Saved docs/src/images/dumbbell.svg");
    Ok(())
}
