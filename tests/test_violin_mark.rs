//! Equivalence and smoke tests for the composite `mark_violin`.
//!
//! The promise of the mark is that it is *only* a name for the public recipe:
//! `transform_density` + `transform_band` + the polygon renderer. The strongest
//! way to keep that promise honest is to render both and compare the bytes.

use charton::prelude::*;
use std::error::Error;
use std::fs;

/// `mark_violin` must reproduce the hand-written density + band recipe exactly,
/// so the two are interchangeable and the sugar can never silently drift away
/// from the grammar it expands to.
#[test]
fn mark_violin_matches_the_manual_recipe_byte_for_byte() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    // --- The composition, written out by hand -----------------------------
    // `mark_violin` adds the original value column's name as the y-axis label;
    // the manual recipe reproduces that with `with_label`.
    chart!(&iris)?
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_trim(true),
        )?
        .transform_band(BandTransform::new("sepal_length", "density"))?
        .mark_polygon()?
        .configure_geoshape(|mark| {
            mark.with_fill("#7fb3d5")
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
                .with_opacity(1.0)
        })
        .encode((
            alt::x("x"),
            alt::y("y").with_label("sepal_length"),
            alt::path_group("path_group"),
        ))?
        .save("./target/test-output/violin_manual.svg")?;

    // --- The same picture through the mark --------------------------------
    chart!(&iris)?
        .mark_violin()?
        .configure_violin(|violin| {
            violin
                .with_color("#7fb3d5")
                .with_stroke("#2c3e50")
                .with_stroke_width(1.0)
                .with_opacity(1.0)
        })
        .encode(alt::y("sepal_length"))?
        .save("./target/test-output/violin_mark.svg")?;

    let manual = fs::read_to_string("./target/test-output/violin_manual.svg")?;
    let sugar = fs::read_to_string("./target/test-output/violin_mark.svg")?;
    assert_eq!(
        manual, sugar,
        "mark_violin must render exactly like transform_density + transform_band"
    );

    Ok(())
}

/// A dodged violin: `x` is the category and `color` splits it into lanes. This
/// exercises the `Position::dodge` branch of the recipe.
#[test]
fn mark_violin_dodged() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    chart!(&penguins)?
        .mark_violin()?
        .encode((
            alt::x("Sex"),
            alt::y("Body Mass (g)"),
            alt::color("Species"),
        ))?
        .save("./target/test-output/violin_mark_dodged.svg")?;

    Ok(())
}

/// A split violin: the same inputs as the dodged form, but the two colour
/// groups share the centre line instead of sitting side by side.
#[test]
fn mark_violin_split() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    chart!(&penguins)?
        .mark_violin()?
        .configure_violin(|violin| violin.with_split(true))
        .encode((
            alt::x("Species"),
            alt::y("Body Mass (g)"),
            alt::color("Sex"),
        ))?
        .save("./target/test-output/violin_mark_split.svg")?;

    Ok(())
}

/// Faceting must re-run the density per panel. If the recipe did not cache the
/// input columns, the second pass would look for the generated `y` column in
/// the raw subset and fail.
#[test]
fn mark_violin_faceted_reruns_the_statistic_per_panel() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    chart!(&penguins)?
        .mark_violin()?
        .encode(alt::y("Body Mass (g)"))?
        .facet(FacetSpec::wrap("Species").with_columns(3))
        .save("./target/test-output/violin_mark_faceted.svg")?;

    Ok(())
}
