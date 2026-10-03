//! Raincloud — a violin outline, an inner box and the raw observations.
//!
//! The raincloud is the clearest demonstration of the layer model: three views
//! of the same distribution, each an ordinary layer, stacked with `.and(…)`.
//!
//! * `transform_violin`      → the density outline
//! * `transform_violin_box`  → the inter-quartile box and median
//! * `mark_point` + jitter   → every observation
//!
//! All three share the same x and y scales, so they line up automatically.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    let params = ViolinTransform::new("Body Mass (g)")
        .with_category("Species")
        .with_scale(ViolinScale::Width);

    // 1. The cloud: a density outline per species.
    let violin = chart!(&penguins)?
        .transform_violin(params.clone())?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("#d6eaf8")
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x").with_category_labels("Species"),
            alt::y("y"),
            alt::path_group("violin_id"),
        ))?;

    // 2. The box: the quartiles, drawn from the same statistics.
    let inner_box = chart!(&penguins)?
        .transform_violin_box(params)?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("white")
                .with_stroke("black")
                .with_stroke_width(1.0)
        })
        .encode((
            alt::x("x").with_category_labels("Species"),
            alt::y("y"),
            alt::path_group("violin_id"),
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
