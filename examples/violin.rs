//! Violin plot — the convenience `mark_violin`.
//!
//! `mark_violin` is a **composite** mark: it expands into the ordinary public
//! recipe `transform_density` → `transform_band` → the shared polygon renderer.
//! For power users who want to build it themselves, `violin_manual.rs` writes the
//! same picture out of those primitives.
//!
//! The inputs come from the encodings:
//!
//! * `y` — the numeric value whose density is drawn (required);
//! * `x` — an optional category each violin sits on;
//! * `color` — an optional group that splits a category into lanes.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        .mark_violin()?
        .configure_violin(|violin| {
            violin
                .with_color("#7fb3d5")
                .with_opacity(0.6)
                .with_stroke("#2c3e50")
        })
        .encode(alt::y("sepal_length"))?
        .with_title("Sepal length")
        .with_x_label("Density")
        .with_y_label("Sepal length (cm)")
        .save("docs/src/images/violin.svg")?;

    println!("Saved docs/src/images/violin.svg");
    Ok(())
}
