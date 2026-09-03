//! Wrap faceting: create one time-series panel for each city.
//!
//! The built-in unemployment dataset contains a complete time series for each
//! country. Every wrapped panel therefore has observations for all years.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("unemployment")?;

    Chart::build(ds)?
        .mark_point()?
        .configure_point(|point| point.with_size(4.0).with_opacity(0.85))
        .encode((alt::x("Year"), alt::y("Unemployment rate (%)")))?
        .facet(
            FacetSpec::wrap("Country")
                .with_columns(4)
                .with_strategy("fixed"),
        )
        .with_title("Unemployment Rate by Country")
        .with_size(1200, 800)
        .save("docs/src/images/facet_wrap.svg")?;

    println!("Saved docs/src/images/facet_wrap.svg");
    Ok(())
}
