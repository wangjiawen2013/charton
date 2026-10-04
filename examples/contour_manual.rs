//! Contour plot — built by hand from the primitives.
//!
//! The same picture as `contour.rs`, without `mark_contour`. A contour is a
//! **composition**, not a dedicated chart type:
//!
//! ```text
//! contour = iso-line stat (marching squares)  +  open-path geometry
//! ```
//!
//! * `transform_contour` (marching squares) turns a gridded `z` into
//!   `(x, y, path_group, level)` polylines;
//! * `mark_path` draws those polylines, coloured by `level`.
//!
//! The scalar grid is *your* data, so `transform_contour` reads `x`, `y`, `z`
//! and replaces the table with `x`, `y`, `path_group`, `level`. In `encode` you
//! then map those columns to channels: `x`/`y` to the axes, `path_group` groups
//! the vertices into separate lines, and `level` colours each line.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Sample z = sin(x) * cos(y) on a regular grid.
    let n = 41;
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut z = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let xv = -3.0 + 6.0 * i as f64 / (n as f64 - 1.0);
            let yv = -3.0 + 6.0 * j as f64 / (n as f64 - 1.0);
            x.push(xv);
            y.push(yv);
            z.push((xv).sin() * (yv).cos());
        }
    }

    chart!(x, y, z)?
        .transform_contour(ContourTransform::new("x", "y", "z").with_levels(10))?
        .mark_path()?
        .configure_path(|m| m.with_stroke_width(1.5))
        .encode((
            alt::x("x"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("level"),
        ))?
        .configure_theme(|t| t.with_color_map(ColorMap::Viridis))
        .with_title("Contour plot")
        .with_x_label("x")
        .with_y_label("y")
        .save("docs/src/images/contour_manual.svg")?;

    println!("Saved docs/src/images/contour_manual.svg");
    Ok(())
}
