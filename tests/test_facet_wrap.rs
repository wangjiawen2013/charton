//! Wrap-faceting smoke test (was `examples/facet_wrap.rs`).

use charton::prelude::*;
use std::error::Error;

#[test]
fn facet_wrap_by_country() -> Result<(), Box<dyn Error>> {
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
        .save("./target/test-output/facet_wrap.svg")?;

    Ok(())
}
