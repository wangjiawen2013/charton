//! A category axis with numeric positions must cover the marks that use it.
//!
//! `with_category_labels` turns a numeric position column into a discrete axis:
//! the ticks sit on the category indices (`0..n-1`). Marks such as a ridgeline
//! place geometry *around* those indices, and an overlapping ridge grows past
//! the last category. The scale now trains on the real position extent as well,
//! so those marks fit without the hand-written `with_expansion` that the
//! recipe used to need.

mod common;

use charton::prelude::*;
use common::{plot_body, plot_rect};
use std::error::Error;

/// Renders the gallery ridgeline, optionally with a manual expansion override.
fn ridge_svg(expansion: Option<Expansion>) -> Result<String, Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    let mut x = alt::x("x").with_category_labels("species");
    if let Some(expansion) = expansion {
        x = x.with_expansion(expansion);
    }

    let svg = chart!(iris)?
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
        .encode((
            x,
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("species"),
        ))?
        .coord_flip()
        .to_svg()?;

    Ok(svg)
}

/// Every point of every mark path inside the plot panel clip group.
fn clipped_mark_points(svg: &str) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    for path in plot_body(svg).split("d=\"").skip(1) {
        let data = path.split('"').next().unwrap_or_default();
        let mut tokens = data.split_whitespace();
        while let Some(command) = tokens.next() {
            if command == "M" || command == "L" {
                let x: f64 = tokens
                    .next()
                    .expect("x coordinate")
                    .parse()
                    .expect("numeric x coordinate");
                let y: f64 = tokens
                    .next()
                    .expect("y coordinate")
                    .parse()
                    .expect("numeric y coordinate");
                points.push((x, y));
            }
        }
    }
    points
}

fn top_of(svg: &str) -> f64 {
    clipped_mark_points(svg)
        .into_iter()
        .map(|(_, y)| y)
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn an_overlapping_ridge_fits_the_panel_without_manual_expansion() -> Result<(), Box<dyn Error>> {
    let svg = ridge_svg(None)?;
    let (px, py, pw, ph) = plot_rect(&svg);
    let points = clipped_mark_points(&svg);
    assert!(!points.is_empty(), "the ridgeline produced no mark points");

    for (x, y) in &points {
        assert!(
            *x >= px - 1e-6 && *x <= px + pw + 1e-6,
            "mark x={x} is outside the panel [{px}, {}]",
            px + pw
        );
        assert!(
            *y >= py - 1e-6 && *y <= py + ph + 1e-6,
            "mark y={y} is outside the panel [{py}, {}]",
            py + ph
        );
    }

    // The top ridge must not merely touch the panel edge: the category axis
    // keeps its padding above the data extent.
    let top = points
        .into_iter()
        .map(|(_, y)| y)
        .fold(f64::INFINITY, f64::min);
    assert!(
        top > py + 1.0,
        "the tallest ridge is flush with the panel top: {top} vs {py}"
    );

    Ok(())
}

#[test]
fn manual_expansion_still_controls_the_headroom() -> Result<(), Box<dyn Error>> {
    let automatic = ridge_svg(None)?;
    let tight = ridge_svg(Some(Expansion {
        mult: (0.0, 0.0),
        add: (0.0, 0.0),
    }))?;
    let generous = ridge_svg(Some(Expansion {
        mult: (0.0, 0.0),
        add: (0.4, 3.0),
    }))?;

    let automatic_top = top_of(&automatic);
    let tight_top = top_of(&tight);
    let generous_top = top_of(&generous);

    // A smaller manual padding removes the extra breathing room but the scale
    // still covers the data extent, so nothing is clipped.
    assert!(
        tight_top < automatic_top,
        "a tighter manual expansion should sit closer to the panel top: \
         automatic {automatic_top}, tight {tight_top}"
    );
    // A larger manual padding adds headroom beyond the automatic fit.
    assert!(
        generous_top > automatic_top + 1.0,
        "a generous manual expansion should leave more headroom: \
         automatic {automatic_top}, generous {generous_top}"
    );

    Ok(())
}

#[test]
fn reversing_a_numeric_category_axis_is_rejected() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    // `sepal_length` is the numeric position, `species` the tick labels. There
    // is no way to reverse the positions by reversing the labels, so the engine
    // reports the unsupported combination instead of mislabelling the marks.
    let result = chart!(iris)?
        .mark_point()?
        .encode((
            alt::x("sepal_length")
                .with_category_labels("species")
                .with_reverse(true),
            alt::y("sepal_width"),
        ))
        .and_then(|chart| chart.to_svg());

    assert!(
        result.is_err(),
        "reversing a category-labelled numeric axis should be rejected"
    );
    Ok(())
}
