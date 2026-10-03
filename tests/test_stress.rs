//! Stress smoke test (was `examples/w.rs`).
//!
//! Renders a few thousand points through the full pipeline to make sure the
//! hot path stays healthy. Output goes to `target/test-output/`.

use charton::error::ChartonError;
use charton::prelude::*;
use std::error::Error;

#[test]
fn stress_many_points() -> Result<(), Box<dyn Error>> {
    let ds = many_points()?;

    Chart::build(&ds)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_title(format!("Performance Test: {} Points", ds.height()))
        .save("./target/test-output/stress_test.svg")?;

    Ok(())
}

/// A couple of thousand points on a sine wave.
fn many_points() -> Result<Dataset, ChartonError> {
    let count = 2_000;
    let x: Vec<f64> = (0..count).map(|i| i as f64 * 0.01).collect();
    let y: Vec<f64> = x.iter().map(|&val| val.sin()).collect();
    Dataset::new().with_column("x", x)?.with_column("y", y)
}
