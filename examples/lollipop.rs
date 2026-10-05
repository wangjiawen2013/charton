//! Lollipop — a bar with the fill thrown away.
//!
//! A recipe: `mark_rule` draws a thin stem from the baseline to each value and
//! `mark_point` caps it. Two ordinary geometries, no new mark. A *range plot*
//! is the same idea with both ends off the baseline (`y` to `y2`).

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let language = [
        "Python",
        "JavaScript",
        "Rust",
        "Go",
        "C++",
        "Java",
        "TypeScript",
        "Ruby",
    ];
    let share = [28.0, 22.0, 12.0, 8.0, 7.0, 6.0, 5.0, 3.0];
    let base = [0.0; 8];

    let stems = chart!(language, share, base)?
        .mark_rule()?
        .configure_rule(|rule| rule.with_color("#b9bfc6").with_stroke_width(2.0))
        .encode((alt::x("language"), alt::y("base"), alt::y2("share")))?;

    let caps = chart!(language, share)?
        .mark_point()?
        .configure_point(|point| point.with_color("#4c78a8").with_size(9.0))
        .encode((alt::x("language"), alt::y("share")))?;

    stems
        .and(caps)
        .coord_flip()
        .with_title("Language popularity")
        .with_x_label("")
        .with_y_label("Share (%)")
        .save("docs/src/images/lollipop.svg")?;

    println!("Saved docs/src/images/lollipop.svg");
    Ok(())
}
