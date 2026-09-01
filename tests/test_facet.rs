use charton::core::layer::Layer;
use charton::prelude::*;
use std::error::Error;

/// Wrap faceting: split the mtcars dataset into panels by `cyl` (number of
/// cylinders).
///
/// Note: `chart.facet(...)` goes through the `IntoLayered` trait and returns a
/// `LayeredChart` directly (not a `Result`), so it must NOT be followed by `?`.
/// The actual row subsetting happens at render time via `with_facet_filter`.
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
/// `render_single_panel -> with_facet_filter` and propagates via `?`.)
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

/// Verify data-subset correctness through `with_facet_filter`:
/// mtcars has 11 rows with `cyl == 4`; after filtering, the subset must have
/// exactly 11 rows and every row must have `cyl == 4`.
#[test]
fn test_facet_filter_subset_correctness() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    let chart = Chart::build(ds.clone())?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?;

    // Manually emulate the filter that a wrap facet on cyl=4 would produce.
    let filter = vec![("cyl".to_string(), "4".to_string())];

    // Invoke through the Layer trait.
    let layer: &dyn Layer = &chart;
    let filtered = layer
        .with_facet_filter(&filter)?
        .expect("A non-empty filter should return Some");

    // The filtered dataset must have exactly the rows where cyl == 4 (11).
    let filtered_ds = filtered.get_dataset();
    assert_eq!(
        filtered_ds.height(),
        11,
        "The cyl=4 subset should have 11 rows, got {}",
        filtered_ds.height()
    );

    // Verify every retained row actually has cyl == 4.
    for row in 0..filtered_ds.height() {
        let cyl = filtered_ds.get("cyl", row).to_string();
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
