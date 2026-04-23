use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    Chart::build(ds)?
        .mark_point()?
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear").with_scale(Scale::Discrete),
        ))?
        .configure_theme(|t| t.with_palette(["#333", "#6fc481", "red"]))
        .save("docs/src/images/custom_palette.svg")?;

    Ok(())
}
