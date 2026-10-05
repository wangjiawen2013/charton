//! Raincloud — a violin outline, an inner box and the raw observations.
//!
//! The raincloud is the clearest demonstration of the layer model: three views
//! of the same distribution, each an ordinary mark, stacked with `.and(…)`.
//!
//! * `mark_violin`              → the density outline
//! * `transform_quantile_box`   → the inter-quartile box and median
//! * `mark_point` + jitter      → every observation
//!
//! All three share the same x and y scales, so they line up automatically.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    // 1. The cloud: a density outline per species, straight from `mark_violin`.
    let violin = chart!(&penguins)?
        .mark_violin()?
        .configure_violin(|mark| {
            mark.with_color("#d6eaf8")
                .with_opacity(1.0)
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
        })
        .encode((alt::x("Species"), alt::y("Body Mass (g)")))?;

    // 2. The box: the quartiles, drawn from the same raw observations.
    let inner_box = chart!(&penguins)?
        .transform_quantile_box(
            QuantileBoxTransform::new("Body Mass (g)").with_category("Species"),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("white")
                .with_stroke("black")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x").with_category_labels("Species"),
            alt::y("y"),
            alt::path_group("path_group"),
        ))?;

    // 3. The rain: every observation, jittered inside the violin.
    let rain = chart!(&penguins)?
        .mark_point()?
        .configure_point(|point| {
            point
                .with_layout("jitter")
                .with_size(2.5)
                .with_opacity(0.55)
        })
        .encode((alt::x("Species"), alt::y("Body Mass (g)")))?;

    violin
        .and(inner_box)
        .and(rain)
        .with_title("Body mass by species (raincloud)")
        .with_x_label("")
        .with_y_label("Body mass (g)")
        .save("docs/src/images/raincloud.svg")?;

    println!("Saved docs/src/images/raincloud.svg");
    Ok(())
}
