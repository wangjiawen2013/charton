//! Equivalence and smoke tests for the composite `mark_density`.
//!
//! As with the other composite marks, the promise is that `mark_density` is only
//! a name for the public `transform_density` + area recipe, so the strongest
//! check is a byte-for-byte comparison against the hand-written composition.

use charton::prelude::*;
use std::error::Error;
use std::fs;

/// `mark_density` must reproduce the hand-written density + area recipe exactly.
#[test]
fn mark_density_matches_the_manual_recipe_byte_for_byte() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    // --- The composition, written out by hand -----------------------------
    chart!(&iris)?
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_groupbys(["species"]),
        )?
        .mark_area()?
        .configure_area(|area| area.with_opacity(0.5))
        .encode((
            alt::x("sepal_length"),
            alt::y("density"),
            alt::color("species"),
        ))?
        .save("./target/test-output/density_manual.svg")?;

    // --- The same picture through the mark --------------------------------
    chart!(&iris)?
        .mark_density()?
        .configure_density(|density| density.with_opacity(0.5))
        .encode((alt::x("sepal_length"), alt::color("species")))?
        .save("./target/test-output/density_mark.svg")?;

    let manual = fs::read_to_string("./target/test-output/density_manual.svg")?;
    let sugar = fs::read_to_string("./target/test-output/density_mark.svg")?;
    assert_eq!(
        manual, sugar,
        "mark_density must render exactly like transform_density + mark_area"
    );

    Ok(())
}

/// A single, ungrouped density curve.
#[test]
fn mark_density_single() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(&iris)?
        .mark_density()?
        .configure_density(|density| density.with_color("#7fb3d5").with_stroke("#2c3e50"))
        .encode(alt::x("sepal_length"))?
        .save("./target/test-output/density_mark_single.svg")?;

    Ok(())
}

/// Faceting must re-run the density estimate per panel.
#[test]
fn mark_density_faceted_reruns_the_statistic_per_panel() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(&iris)?
        .mark_density()?
        .encode(alt::x("sepal_length"))?
        .facet(FacetSpec::wrap("species").with_columns(3))
        .save("./target/test-output/density_mark_faceted.svg")?;

    Ok(())
}
