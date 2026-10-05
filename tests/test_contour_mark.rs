//! Equivalence and smoke tests for the composite `mark_contour`.
//!
//! As with `mark_violin`, the mark must be a name for the public recipe and
//! nothing more, so the strongest check is a byte-for-byte comparison against
//! the hand-written `transform_contour` + `mark_path` composition.

use charton::prelude::*;
use std::error::Error;
use std::fs;

/// The same `z = sin(x)·cos(y)` grid used by `examples/contour.rs`.
fn sin_cos_grid() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
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
            z.push(xv.sin() * yv.cos());
        }
    }
    (x, y, z)
}

/// `mark_contour` must reproduce the hand-written iso-line recipe exactly.
#[test]
fn mark_contour_matches_the_manual_recipe_byte_for_byte() -> Result<(), Box<dyn Error>> {
    let (x, y, z) = sin_cos_grid();

    // --- The composition, written out by hand -----------------------------
    // The mark titles the axes with the original grid columns and the colour
    // legend with the scalar field; the manual recipe reproduces that with
    // `with_label`.
    chart!(x, y, z)?
        .transform_contour(ContourTransform::new("x", "y", "z").with_levels(10))?
        .mark_path()?
        .configure_path(|mark| mark.with_stroke_width(1.5))
        .encode((
            alt::x("x"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("level").with_label("z"),
        ))?
        .configure_theme(|theme| theme.with_color_map(ColorMap::Viridis))
        .save("./target/test-output/contour_manual.svg")?;

    // --- The same picture through the mark --------------------------------
    chart!(x, y, z)?
        .mark_contour("z")?
        .configure_contour(|contour| contour.with_levels(10).with_stroke_width(1.5))
        .encode((alt::x("x"), alt::y("y")))?
        .configure_theme(|theme| theme.with_color_map(ColorMap::Viridis))
        .save("./target/test-output/contour_mark.svg")?;

    let manual = fs::read_to_string("./target/test-output/contour_manual.svg")?;
    let sugar = fs::read_to_string("./target/test-output/contour_mark.svg")?;
    assert_eq!(
        manual, sugar,
        "mark_contour must render exactly like transform_contour + mark_path"
    );

    Ok(())
}

/// Single-colour contour: no colour-by-level, so the outline colour is used.
#[test]
fn mark_contour_single_colour() -> Result<(), Box<dyn Error>> {
    let (x, y, z) = sin_cos_grid();

    chart!(x, y, z)?
        .mark_contour("z")?
        .configure_contour(|contour| {
            contour
                .with_levels(12)
                .with_color_by_level(false)
                .with_stroke("black")
                .with_stroke_width(1.0)
        })
        .encode((alt::x("x"), alt::y("y")))?
        .save("./target/test-output/contour_mark_single.svg")?;

    Ok(())
}

/// The generated columns are named `x`/`y`, but the axes must keep the
/// original grid column names. This uses columns deliberately *not* called
/// `x`/`y` so that only the label mechanism can produce the right titles.
#[test]
fn mark_contour_keeps_the_original_axis_names() -> Result<(), Box<dyn Error>> {
    let n = 11;
    let mut lon = Vec::new();
    let mut lat = Vec::new();
    let mut height = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let xv = i as f64;
            let yv = j as f64;
            lon.push(xv);
            lat.push(yv);
            height.push(xv * yv);
        }
    }

    chart!(lon, lat, height)?
        .mark_contour("height")?
        .configure_contour(|contour| contour.with_levels(5))
        .encode((alt::x("lon"), alt::y("lat")))?
        .save("./target/test-output/contour_mark_names.svg")?;

    let svg = fs::read_to_string("./target/test-output/contour_mark_names.svg")?;
    assert!(
        svg.contains(">lon</text>"),
        "x axis should be titled with the original column name"
    );
    assert!(
        svg.contains(">lat</text>"),
        "y axis should be titled with the original column name"
    );
    assert!(
        svg.contains(">height</text>"),
        "the colour legend should be titled with the original scalar field"
    );

    Ok(())
}

/// The bivariate-density flow: `transform_density_2d` produces the grid, then
/// `mark_contour("density")` contours it.
#[test]
fn mark_contour_from_density_2d() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(&iris)?
        .transform_density_2d(
            Density2DTransform::new("sepal_length", "petal_length")
                .with_grid_size(60)
                .with_padding(0.3),
        )?
        .mark_contour("density")?
        .encode((alt::x("x"), alt::y("y")))?
        .save("./target/test-output/contour_mark_density.svg")?;

    Ok(())
}
