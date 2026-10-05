//! Bump chart — how a ranking changes over time.
//!
//! Ranks are a statistic, not a new geometry: `transform_window` with
//! `WindowOnlyOp::Rank` ranks each country inside every year, and ordinary
//! `mark_line`/`mark_point` layers draw the trajectories. The y-axis is
//! reversed (`with_reverse(true)`) so rank 1 sits at the top.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let unemployment = load_dataset("unemployment")?;
    let rank = || {
        WindowTransform::new(WindowFieldDef::new(
            "Unemployment rate (%)",
            WindowOnlyOp::Rank,
            "rank",
        ))
        .with_groupbys(["Year"])
    };

    let lines = chart!(&unemployment)?
        .transform_window(rank())?
        .mark_line()?
        .configure_line(|line| line.with_stroke_width(3.0))
        .encode((
            alt::x("Year"),
            alt::y("rank").with_reverse(true),
            alt::color("Country"),
        ))?;

    let dots = chart!(&unemployment)?
        .transform_window(rank())?
        .mark_point()?
        .configure_point(|point| point.with_size(7.0))
        .encode((
            alt::x("Year"),
            alt::y("rank").with_reverse(true),
            alt::color("Country"),
        ))?;

    lines
        .and(dots)
        .with_title("Unemployment rank by year")
        .with_x_label("")
        .with_y_label("Rank")
        .save("docs/src/images/bump.svg")?;

    println!("Saved docs/src/images/bump.svg");
    Ok(())
}
