use charton::core::layer::Layer;
use charton::prelude::*;
use std::collections::HashSet;
use std::error::Error;

/// Wrap faceting: split the mtcars dataset into panels by `cyl` (number of
/// cylinders).
///
/// Note: `chart.facet(...)` goes through the `IntoLayered` trait and returns a
/// `LayeredChart` directly (not a `Result`), so it must NOT be followed by `?`.
/// The actual row subsetting happens at render time via the layer's
/// `facet_partition` / `subset_rows` contract.
#[test]
fn test_facet_wrap_cyl() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    // Before faceting: full 32 rows.
    assert_eq!(ds.height(), 32, "mtcars should contain 32 rows");

    // Apply a mark first, then facet (facet lives on the IntoLayered trait).
    let chart = Chart::build(ds.clone())?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?;

    let lc = chart.facet("cyl");

    let svg = lc.to_svg()?;
    assert!(
        !svg.is_empty(),
        "A faceted chart should export a non-empty SVG"
    );

    Ok(())
}

/// Grid faceting: split mtcars into a row x column matrix of panels using
/// `cyl` (rows) x `gear` (columns).
#[test]
fn test_facet_grid_cyl_gear() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    let chart = Chart::build(ds.clone())?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?;

    let lc = chart.facet(("cyl", "gear"));

    let svg = lc.to_svg()?;
    assert!(
        !svg.is_empty(),
        "A grid-faceted chart should export a non-empty SVG"
    );

    Ok(())
}

#[test]
fn test_facet_svg_uses_unique_clip_paths() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    let svg = Chart::build(ds)?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?
        .facet(FacetSpec::grid("vs", "am"))
        .to_svg()?;

    let clip_ids: HashSet<&str> = svg
        .split("clipPath id=\"")
        .skip(1)
        .filter_map(|part| part.split('"').next())
        .collect();
    let clip_refs = svg.matches("clip-path=\"url(#").count();

    assert_eq!(clip_ids.len(), 4, "each grid panel needs its own clipPath");
    assert_eq!(clip_refs, 4, "each grid panel needs its own clip reference");

    Ok(())
}

#[test]
fn test_multi_panel_shows_grid_by_default_but_can_be_disabled() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    let chart = Chart::build(ds.clone())?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?
        .facet(FacetSpec::grid("vs", "am"));
    let default_svg = chart.to_svg()?;

    let disabled_svg = Chart::build(ds)?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?
        .facet(FacetSpec::grid("vs", "am"))
        .with_grid(false)
        .to_svg()?;

    assert!(
        default_svg.matches("stroke-opacity=\"0.500\"").count() > 0,
        "multi-panel charts should show grid lines by default"
    );
    assert_eq!(
        disabled_svg.matches("stroke-opacity=\"0.500\"").count(),
        0,
        "explicit with_grid(false) should disable grid lines"
    );

    Ok(())
}

/// Wrap faceting with an explicit column count and a free-axis strategy.
/// (`FacetSpec` is now re-exported from the prelude.)
#[test]
fn test_facet_wrap_with_columns_and_strategy() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    let chart = Chart::build(ds.clone())?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?;

    let lc = chart.facet(FacetSpec::wrap("cyl").with_columns(2).with_strategy("free"));

    let svg = lc.to_svg()?;
    assert!(
        !svg.is_empty(),
        "Faceting with columns and strategy should export SVG"
    );

    Ok(())
}

/// Fail-fast guard: when a facet field is missing from the data, rendering
/// must error out instead of silently drawing the full dataset. (The facet
/// call itself does not validate; validation happens in
/// `render_single_panel -> facet_partition` and propagates via `?`.)
#[test]
fn test_facet_missing_field_errors() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    let chart = Chart::build(ds.clone())?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?;

    // "not_a_column" does not exist; the facet call succeeds (IntoLayered does
    // not validate), but rendering fails fast when the field is missing.
    let lc = chart.facet("not_a_column");

    let result = lc.to_svg();
    assert!(
        result.is_err(),
        "Rendering should error when the facet field is missing"
    );

    Ok(())
}

/// Verify data-subset correctness through the faceting primitives:
/// mtcars has 11 rows with `cyl == 4`; after subsetting, the layer must have
/// exactly 11 rows and every row must have `cyl == 4`.
#[test]
fn test_facet_subset_correctness() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    let chart = Chart::build(ds.clone())?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?;

    // Emulate the two-step faceting contract: build the partition once, then
    // resolve a panel's rows from it.
    let layer: &dyn Layer = &chart;
    let partition = layer
        .facet_partition(&["cyl"])?
        .expect("A data layer should return a partition");

    let filter = vec![("cyl".to_string(), "4".to_string())];
    let rows = partition
        .row_indices(&filter)
        .expect("The cyl=4 group should exist");

    // The cyl=4 sub-population has exactly 11 rows.
    assert_eq!(rows.len(), 11, "The cyl=4 group should have 11 rows");

    // Materialize the subset layer and verify every retained row has cyl == 4.
    let subset = layer
        .subset_rows(rows)?
        .expect("A non-empty subset should return Some");

    let subset_ds = subset.get_dataset();
    assert_eq!(
        subset_ds.height(),
        11,
        "The cyl=4 subset should have 11 rows, got {}",
        subset_ds.height()
    );

    for row in 0..subset_ds.height() {
        let cyl = subset_ds.get("cyl", row).to_string();
        assert_eq!(
            cyl.as_deref(),
            Some("4"),
            "Row {} should have cyl == 4, got {:?}",
            row,
            cyl
        );
    }

    Ok(())
}
