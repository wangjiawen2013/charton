use charton::prelude::*;
use std::error::Error;

#[test]
fn test_palette_1() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    Chart::build(ds)?
        .mark_point()?
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear").with_scale(Scale::Discrete),
        ))?
        .configure_theme(|t| t.with_palette(["#333", "#6fc481", "red"]))
        .save("./tests/palette1.svg")?;

    Ok(())
}

#[test]
fn test_palette_2() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    Chart::build(ds)?
        .mark_point()?
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear").with_scale(Scale::Discrete),
        ))?
        .configure_theme(|t| {
            t.with_palette(vec!["#ff0000", "rgba(0,0,255,1.0)", "rgb(100, 100, 100)"])
        })
        .save("./tests/palette2.svg")?;

    Ok(())
}

#[test]
fn test_palette_3() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    Chart::build(ds)?
        .mark_point()?
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear").with_scale(Scale::Discrete),
        ))?
        .configure_theme(|t| t.with_palette(vec![SingleColor::none(), SingleColor::new("red")]))
        .save("./tests/palette3.svg")?;

    Ok(())
}
