//! Contour plot — the convenience `mark_contour`.
//!
//! `mark_contour` is a **composite** mark: it expands into the public
//! `transform_contour` (marching squares over a regular grid) and the shared
//! open-path renderer. For the composition written out by hand, see
//! `contour_manual.rs`.
//!
//! The scalar field `z` is named explicitly because the crate has no `z`
//! channel; `x` and `y` come from the encodings. By default the lines are
//! coloured by their level.

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
        .mark_contour("z")?
        .configure_contour(|contour| contour.with_levels(10).with_stroke_width(1.5))
        .encode((alt::x("x"), alt::y("y")))?
        .configure_theme(|t| t.with_color_map(ColorMap::Viridis))
        .with_title("Contour plot")
        .with_x_label("x")
        .with_y_label("y")
        .save("docs/src/images/contour.svg")?;

    println!("Saved docs/src/images/contour.svg");
    Ok(())
}
