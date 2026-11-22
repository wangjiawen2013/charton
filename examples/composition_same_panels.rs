//! Four panels, one panel size: `Align::All`.
//!
//! ```text
//!            column 0                 column 1
//!          +------------------------+------------------------+
//!  row 0   |  box plot              |  violin plot           |
//!          +------------------------+------------------------+
//!  row 1   |  histogram             |  scatter               |
//!          +------------------------+------------------------+
//! ```
//!
//! # What it demonstrates
//!
//! * **`Align::All` gives every cell one data rectangle for the whole page** — not
//!   one per track. The four charts here carry *different* furniture (one has a long
//!   y-axis title, one has a colour legend in its own band, one hides its y axis), so
//!   their natural panels would come out at four different sizes — and the scatter's
//!   legend is hoisted out of its cell, which changes that panel again. `All`
//!   intersects them into a single rectangle, and every panel lands on the same
//!   lines.
//! * **`Resolve::Independent` is the honest partner of `All` here.** These four
//!   charts measure different things, so they share the rectangle but not the scales:
//!   the same pixels mean different values in each panel. (With `Resolve::Shared`
//!   every panel would be forced onto one domain — right for small multiples of the
//!   *same* measure, wrong for four different questions.)
//! * **`All` collapses onto the narrowest panel.** The shared rectangle is the
//!   *intersection*, so the panel widest in nature is trimmed to match the tightest
//!   one. That is the price of "one size for everybody", and the reason to use equal
//!   tracks (as here) when you ask for it.
//!
//! Compare `composition_dashboard.rs`, which uses `Align::Each` because its tracks
//! are deliberately unequal, and `composition_linked_panels.rs`, where two panels
//! share one axis on purpose.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    // Deliberately different furniture: a long y title, a legend, hidden axes.
    let boxes = chart!(&iris)?
        .mark_boxplot()?
        .encode((alt::x("species"), alt::y("petal_length")))?
        .with_y_label("petal length (cm)");

    let violins = chart!(&iris)?
        .mark_violin()?
        .configure_violin(|violin| violin.with_color("#7fb3d5").with_opacity(0.7))
        .encode((alt::x("species"), alt::y("petal_width")))?
        .with_y_label("petal width (cm)");

    let histogram = chart!(&iris)?
        .mark_hist()?
        .encode((alt::x("sepal_length").with_bins(20), alt::y("count")))?
        .with_y_label("");

    let scatter = chart!(&iris)?
        .mark_point()?
        .encode((
            alt::x("petal_length"),
            alt::y("petal_width"),
            alt::color("species"),
        ))?
        .configure_point(|point| point.with_size(4.0).with_opacity(0.8));

    Composition::grid(2, 2)
        .with_cell(0, 0, boxes)
        .with_cell(0, 1, violins)
        .with_cell(1, 0, histogram)
        .with_cell(1, 1, scatter)
        // One rectangle, four panels.
        .with_align(Align::All)
        // ...but four independent value ranges.
        .with_resolve(Resolve::Independent)
        .with_spacing(14.0)
        .with_legend_position(LegendPosition::Right)
        .with_legend_from(1, 1)
        .with_title("Iris, four ways")
        .with_size(760, 600)
        .save("docs/src/images/composition_same_panels.svg")?;

    println!("Saved docs/src/images/composition_same_panels.svg");
    Ok(())
}
