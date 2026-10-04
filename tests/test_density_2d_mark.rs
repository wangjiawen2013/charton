//! Equivalence and smoke tests for the composite `mark_density_2d`.
//!
//! `mark_density_2d` must be a name for the public `transform_density_2d` +
//! `mark_rect` recipe (plus the automatic bin alignment), so the strongest check
//! is a byte-for-byte comparison against the hand-written composition.

use charton::prelude::*;
use std::error::Error;
use std::fs;

/// `mark_density_2d` must reproduce the hand-written density-heatmap recipe
/// exactly, including the `with_bins(grid_size)` alignment it does for you.
#[test]
fn mark_density_2d_matches_the_manual_recipe_byte_for_byte() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    // --- The composition, written out by hand -----------------------------
    chart!(&iris)?
        .transform_density_2d(
            Density2DTransform::new("sepal_length", "petal_length")
                .with_grid_size(60)
                .with_padding(0.2),
        )?
        .mark_rect()?
        .encode((
            alt::x("x").with_bins(60).with_label("sepal_length"),
            alt::y("y").with_bins(60).with_label("petal_length"),
            alt::color("density"),
        ))?
        .configure_theme(|theme| theme.with_color_map(ColorMap::Viridis))
        .save("./target/test-output/density_2d_manual.svg")?;

    // --- The same picture through the mark --------------------------------
    chart!(&iris)?
        .mark_density_2d()?
        .configure_density_2d(|density| density.with_grid_size(60).with_padding(0.2))
        .encode((alt::x("sepal_length"), alt::y("petal_length")))?
        .configure_theme(|theme| theme.with_color_map(ColorMap::Viridis))
        .save("./target/test-output/density_2d_mark.svg")?;

    let manual = fs::read_to_string("./target/test-output/density_2d_manual.svg")?;
    let sugar = fs::read_to_string("./target/test-output/density_2d_mark.svg")?;
    assert_eq!(
        manual, sugar,
        "mark_density_2d must render exactly like transform_density_2d + mark_rect"
    );

    Ok(())
}

/// Default configuration (no grid/padding overrides) renders without panicking
/// and keeps the original axis names.
#[test]
fn mark_density_2d_defaults() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(&iris)?
        .mark_density_2d()?
        .encode((alt::x("sepal_length"), alt::y("petal_length")))?
        .save("./target/test-output/density_2d_mark_default.svg")?;

    let svg = fs::read_to_string("./target/test-output/density_2d_mark_default.svg")?;
    assert!(svg.contains(">sepal_length</text>"));
    assert!(svg.contains(">petal_length</text>"));

    Ok(())
}
