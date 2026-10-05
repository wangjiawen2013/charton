//! Slope chart — change between two points in time.
//!
//! A recipe: `mark_line` connects each country's two values across the two
//! periods and `mark_point` marks them. Grouping the line by colour is all it
//! takes; the two layers share the category and value scales.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
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

    let countries_long: Vec<&str> = country.iter().chain(country.iter()).copied().collect();
    let periods: Vec<&str> = std::iter::repeat("2011")
        .take(7)
        .chain(std::iter::repeat("2021").take(7))
        .collect();
    let values: Vec<f64> = y2011.iter().chain(y2021.iter()).copied().collect();

    let ds = Dataset::new()
        .with_column("country", countries_long)?
        .with_column("period", periods)?
        .with_column("rate", values)?;

    let lines = chart!(&ds)?
        .mark_line()?
        .configure_line(|line| line.with_stroke_width(2.0))
        .encode((alt::x("period"), alt::y("rate"), alt::color("country")))?;

    let dots = chart!(&ds)?
        .mark_point()?
        .configure_point(|point| point.with_size(7.0))
        .encode((alt::x("period"), alt::y("rate"), alt::color("country")))?;

    lines
        .and(dots)
        .with_title("Unemployment rate, 2011 vs 2021")
        .with_x_label("")
        .with_y_label("Unemployment rate (%)")
        .save("docs/src/images/slope.svg")?;

    println!("Saved docs/src/images/slope.svg");
    Ok(())
}
