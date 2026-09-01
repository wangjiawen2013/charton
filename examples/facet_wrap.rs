//! Wrap faceting: split a chart into a grid of panels, one per category.
//!
//! The `facet` method accepts a `FacetSpec`, and `&str` is implicitly converted
//! into a wrap facet on a single field. Here we facet the mtcars dataset by
//! `cyl` (number of cylinders): each panel is a scatter of `wt` vs `mpg` for a
//! single cylinder count, so the global encoding and mark are defined once and
//! reused across all panels.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    // chart.facet("cyl")  ==  chart.facet(FacetSpec::wrap("cyl"))
    Chart::build(ds)?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?
        .facet("cyl")
        .save("docs/src/images/facet_wrap.svg")?;

    println!("Saved docs/src/images/facet_wrap.svg");
    Ok(())
}
