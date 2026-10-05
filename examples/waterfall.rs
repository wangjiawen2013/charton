//! Waterfall — a running total shown as floating bars.
//!
//! Two public primitives do the work: `transform_window` with
//! `WindowOnlyOp::CumulativeSum` carries the running total, and `mark_bar` with
//! a `y2` bound draws each step as a bar that floats between the total *before*
//! and *after* it. The opening and closing **totals** are the same idea anchored
//! at zero (`base = 0`), so the picture needs one geometry, not two.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Opening total, quarterly contributions, closing total.
    let step = ["Start", "Q1", "Q2", "Q3", "Q4", "End"];
    let delta = [200.0, 50.0, 30.0, -20.0, -10.0, 0.0];
    let kind = [
        "Total", "Increase", "Increase", "Decrease", "Decrease", "Total",
    ];

    chart!(step, delta, kind)?
        // The running total after each step ...
        .transform_window(WindowTransform::new(WindowFieldDef::new(
            "delta",
            WindowOnlyOp::CumulativeSum,
            "total",
        )))?
        // ... and the bar's lower edge: the total before the step, or zero for
        // the opening/closing total columns.
        .transform_calculate("base", |row| {
            if row.str("kind").as_deref() == Some("Total") {
                Some(0.0)
            } else {
                Some(row.val("total")? - row.val("delta")?)
            }
        })?
        .mark_bar()?
        .configure_bar(|bar| bar.with_width(0.6))
        .encode((
            alt::x("step"),
            alt::y("base"),
            alt::y2("total"),
            alt::color("kind"),
        ))?
        .with_title("Running total")
        .with_x_label("")
        .with_y_label("Value")
        .with_color_label("")
        .save("docs/src/images/waterfall.svg")?;

    println!("Saved docs/src/images/waterfall.svg");
    Ok(())
}
