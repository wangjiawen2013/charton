//! Split violin — two groups as the left and right halves of one violin, with
//! `mark_violin`.
//!
//! Set `with_split(true)` and colour by the group: the first group grows to the
//! right of each centre and the second to the left. The mark expands this into
//! the ordinary density transform plus the general band geometry in `split`
//! mode, so each half is an ordinary polygon.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    chart!(&penguins)?
        .mark_violin()?
        .configure_violin(|violin| {
            violin
                .with_split(true)
                .with_color("#95a5a6")
                .with_stroke("#2c3e50")
        })
        .encode((
            alt::x("Species"),
            alt::y("Body Mass (g)"),
            alt::color("Sex"),
        ))?
        .with_title("Body mass by species and sex (split)")
        .with_x_label("")
        .with_y_label("Body mass (g)")
        .save("docs/src/images/split_violin.svg")?;

    println!("Saved docs/src/images/split_violin.svg");
    Ok(())
}
