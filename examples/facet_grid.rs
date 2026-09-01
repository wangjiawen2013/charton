//! Grid faceting: split a chart into a row x column matrix of panels.
//!
//! A tuple `("row_field", "col_field")` is implicitly converted into a grid
//! facet. Here we facet the mtcars dataset by `cyl` (rows) and `gear`
//! (columns), producing one scatter panel (wt vs mpg) for every combination of
//! cylinder count and number of forward gears.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    // chart.facet(("cyl", "gear"))  ==  chart.facet(FacetSpec::grid("cyl", "gear"))
    Chart::build(ds)?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?
        .facet(("cyl", "gear"))
        .save("docs/src/images/facet_grid.svg")?;

    println!("Saved docs/src/images/facet_grid.svg");
    Ok(())
}
