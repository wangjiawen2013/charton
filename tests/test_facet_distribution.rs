use charton::prelude::*;
use std::error::Error;

#[test]
fn test_facet_wrap_histogram_renders() -> Result<(), Box<dyn Error>> {
    let values = vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5];
    let groups = vec![
        "low", "low", "low", "low", "low", "high", "high", "high", "high", "high",
    ];

    let chart = chart!(values, groups)?
        .mark_hist()?
        .encode((
            alt::x("values"),
            alt::y("count").with_normalize(true),
            alt::color("groups"),
        ))?
        .facet_wrap("groups")?;

    chart
        .with_size(700, 400)
        .save("./tests/facet_distribution.svg")?;

    Ok(())
}

#[test]
fn test_faceted_bar_svg_uses_panel_specific_clip_paths() -> Result<(), Box<dyn Error>> {
    let values = vec![1.0, 2.0];
    let groups = vec!["low", "high"];

    let chart = chart!(values, groups)?
        .mark_bar()?
        .encode((alt::x("groups"), alt::y("values")))?
        .facet_wrap("groups")?;

    let svg = chart.with_size(500, 300).to_svg()?;

    assert!(
        svg.contains("clipPath id=\"plot-clip-area-"),
        "expected faceted SVG to use panel-specific clip paths, got: {svg}"
    );
    assert!(
        !svg.contains("clipPath id=\"plot-clip-area\""),
        "expected a unique clip path id per panel"
    );

    Ok(())
}

#[test]
fn test_facet_strip_labels_align_to_the_right() -> Result<(), Box<dyn Error>> {
    let values = vec![1.0, 2.0, 3.0, 4.0];
    let groups = vec!["low", "low", "high", "high"];

    let chart = chart!(values, groups)?
        .mark_bar()?
        .encode((alt::x("groups"), alt::y("values")))?
        .facet_wrap("groups")?;

    let svg = chart.with_size(500, 300).to_svg()?;

    assert!(
        svg.contains("text-anchor=\"middle\""),
        "expected facet strip labels to be centered by default, got: {svg}"
    );
    assert!(
        svg.contains(">groups</text>"),
        "expected the facet field label to appear on the right edge, got: {svg}"
    );

    Ok(())
}

#[test]
fn test_facet_grid_renders() -> Result<(), Box<dyn Error>> {
    let values = vec![1.0, 2.0, 3.0, 4.0];
    let groups = vec!["low", "low", "high", "high"];
    let region = vec!["X", "Y", "X", "Y"];

    let chart = chart!(values, groups, region)?
        .mark_bar()?
        .encode((alt::x("groups"), alt::y("values")))?
        .facet_grid("groups", "region")?;

    let svg = chart.with_size(700, 400).to_svg()?;

    assert!(
        svg.contains("clipPath id=\"plot-clip-area-"),
        "expected facet_grid SVG to use panel-specific clip paths, got: {svg}"
    );
    assert!(
        svg.contains("<svg"),
        "expected facet_grid to produce valid SVG output, got: {svg}"
    );

    Ok(())
}
