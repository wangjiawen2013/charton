//! Violin plot — the basic, single-distribution form.
//!
//! A violin draws the *density* of a numeric column as a symmetric outline.
//! The shape is produced by three ordinary parts, not by a special "violin
//! mark":
//!
//! 1. `transform_violin` estimates a density curve and mirrors it into a closed
//!    polygon (the statistical part).
//! 2. `mark_polygon` draws that polygon (the geometric part, the same renderer
//!    used by maps and any other outline).
//! 3. `encode` says which output columns hold the coordinates.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        // Estimate the density of sepal length and turn it into an outline.
        .transform_violin(ViolinTransform::new("sepal_length"))?
        // Any polygon renderer works; here we use the shared one.
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("#7fb3d5")
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
        })
        // The transform wrote three columns: x, y and a group id per violin.
        .encode((alt::x("x"), alt::y("y"), alt::path_group("violin_id")))?
        .with_title("Sepal length")
        .with_x_label("Density")
        .with_y_label("Sepal length (cm)")
        .save("docs/src/images/violin.svg")?;

    println!("Saved docs/src/images/violin.svg");
    Ok(())
}
