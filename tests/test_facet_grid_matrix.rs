use charton::prelude::*;
use std::error::Error;

#[test]
fn test_facet_grid_matrix_svg_exports() -> Result<(), Box<dyn Error>> {
    let values = vec![1.0, 2.0, 3.0, 4.0];
    let groups = vec!["low", "low", "high", "high"];
    let region = vec!["north", "south", "north", "south"];

    let chart = chart!(values, groups, region)?
        .mark_bar()?
        .encode((alt::x("groups"), alt::y("values")))?
        .facet_grid("groups", "region")?;

    let svg = chart.clone().with_size(700, 400).to_svg()?;

    assert!(
        svg.contains("clipPath id=\"plot-clip-area-"),
        "expected facet_grid SVG to use panel-specific clip paths, got: {svg}"
    );
    assert!(
        svg.contains("<svg"),
        "expected facet_grid matrix export to produce valid SVG output, got: {svg}"
    );

    chart
        .with_size(700, 400)
        .save("./tests/facet_grid_matrix.svg")?;

    Ok(())
}
