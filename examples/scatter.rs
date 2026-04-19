use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    Chart::build(ds)?
        .mark_point()?
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear"),
            alt::shape("gear"),
            alt::size("mpg"),
        ))?
        .coord_flip()
        .configure_theme(|t| t.with_x_tick_label_angle(-45.0))
        .with_title("abc")
        .save("docs/src/images/scatter.svg")?;

    Ok(())
}
