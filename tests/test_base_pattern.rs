//! The base-pattern smoke test (was `examples/base.rs`).
//!
//! One `Chart<NoMark>` base is specialized into a line *and* a point layer and
//! layered together, demonstrating the "one base, many marks" pattern.

use charton::prelude::*;
use std::error::Error;

#[test]
fn base_pattern_specializes_one_base_into_layers() -> Result<(), Box<dyn Error>> {
    let length = vec![4.4, 4.6, 4.7, 4.9, 5.0, 5.1, 5.4];
    let width = vec![2.9, 3.1, 3.2, 3.0, 3.6, 3.5, 3.9];

    let base = chart!(length, width)?.encode((alt::x("length"), alt::y("width")))?;

    let line = base.clone().mark_line()?;
    let scatter = base.mark_point()?;

    line.and(scatter).save("./target/test-output/base.svg")?;

    Ok(())
}
