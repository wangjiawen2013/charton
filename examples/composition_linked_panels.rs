//! Two panels that share an axis: the linked-scales layout.
//!
//! ```text
//!            column 0
//!          +-------------------------+
//!  row 0   |  petal length × sepal length   (no x axis: the row below has it)
//!          +-------------------------+
//!  row 1   |  petal length × petal width
//!          +-------------------------+
//! ```
//!
//! # What it demonstrates
//!
//! * **What a column shares and what a row shares.** `Align::Each` gives a column
//!   one **x** rectangle and a row one **y** rectangle. These two views are in the
//!   same column and different rows, so they line up horizontally and keep their own
//!   vertical ranges — which is exactly what "linked x, independent y" means.
//! * **`Resolve::Shared` is why the x ranges agree**, not just the rectangles: the
//!   top panel adopts the bottom one's domain (they share the `petal_length` field,
//!   so it is the same range anyway — but the mechanism is what guarantees it when
//!   the two views hold different rows of data).
//! * **One axis for two panels.** `with_show_x_axis(false)` on the upper view frees
//!   its band; repeating the tick labels would draw the same numbers twice.
//! * **A tight row gap.** `with_row_spacing(2.0)` is what makes the pair read as one
//!   figure — matplotlib's `hspace`.
//!
//! No statistic, no new mark: two ordinary scatters whose layout is decided here.
//! `composition_same_panels.rs` is the sibling example where the two panels
//! deliberately do **not** share a scale.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    // Upper panel: the same x, a different y. Its x axis is the one below.
    let upper = chart!(&iris)?
        .mark_point()?
        .encode((alt::x("petal_length"), alt::y("sepal_length")))?
        .configure_point(|point| point.with_size(4.0).with_opacity(0.6))
        .with_show_x_axis(false);

    let lower = chart!(&iris)?
        .mark_point()?
        .encode((alt::x("petal_length"), alt::y("petal_width")))?
        .configure_point(|point| point.with_size(4.0).with_opacity(0.6));

    Composition::vconcat((upper, lower))
        .with_row_height(0, TrackSize::fill(1.0))
        .with_row_height(1, TrackSize::fill(1.0))
        // One x rectangle per column, one y rectangle per row.
        .with_align(Align::Each)
        .with_row_spacing(2.0)
        .with_title("Two measures against one petal length")
        .with_size(600, 480)
        .save("docs/src/images/composition_linked_panels.svg")?;

    println!("Saved docs/src/images/composition_linked_panels.svg");
    Ok(())
}
