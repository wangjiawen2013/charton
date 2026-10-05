//! Ridgeline — one density per category, each sitting on its baseline.
//!
//! This is a composition, not a mark. The density comes from
//! `transform_density`; `transform_band(…).with_side(BandSide::Right)` turns each
//! curve into a one-sided band whose flat edge is the category baseline; and
//! `coord_flip` puts the value on the horizontal axis and the density bulge on
//! the vertical one. `with_overlap` lets each ridge grow past its lane so it
//! overlaps the one above it. A ridge is a violin that keeps one bank.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_groupbys(["species"]),
        )?
        .transform_band(
            BandTransform::new("sepal_length", "density")
                .with_center("species")
                .with_side(BandSide::Right)
                .with_width(1.0)
                .with_overlap(2.5)
                .with_scale(BandScale::PerGroup),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_opacity(0.85)
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
        })
        .encode((
            // Each ridge grows one and a quarter categories past its baseline,
            // so the category axis needs headroom above the top one (a plain
            // violin only needs the default 0.4).
            alt::x("x")
                .with_category_labels("species")
                .with_expansion(Expansion {
                    mult: (0.0, 0.0),
                    add: (0.4, 1.5),
                }),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("species"),
        ))?
        .coord_flip()
        .with_title("Sepal length by species")
        .with_x_label("Sepal length (cm)")
        .with_y_label("")
        .save("docs/src/images/ridge.svg")?;

    println!("Saved docs/src/images/ridge.svg");
    Ok(())
}
