//! Density plot — the convenience `mark_density`.
//!
//! `mark_density` is a **composite** mark: it expands into the public
//! `transform_density` and the shared area geometry. For the composition written
//! out by hand, see `density_manual.rs`. It is the smooth counterpart of a
//! histogram; a violin is the same curve stood upright and mirrored.

use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ds = load_dataset("iris")?;

    chart!(ds)?
        .mark_density()?
        .configure_density(|density| density.with_opacity(0.5))
        .encode((alt::x("sepal_length"), alt::color("species")))?
        .save("docs/src/images/density.svg")?;

    Ok(())
}
