//! A dashboard: three views of one table, arranged by hand.
//!
//! The marginal examples use `Composition` for one specific figure. This one uses
//! it the other way round — as a general page layout — and is the example to read
//! first if you are not drawing marginals:
//!
//! ```text
//!              column 0 (Fill 1)        column 1 (Fill 1)
//!            +------------------------+------------------+
//!   row 0    |  bar: mean sepal width, spanning both       |   Fixed 200
//!            +------------------------+------------------+
//!   row 1    |  histogram: petal len  |  scatter          |   Fill
//!            +------------------------+------------------+
//! ```
//!
//! # What it demonstrates
//!
//! * **`hconcat` / `vconcat` are shorthands, a grid is the general form.** A
//!   dashboard that is not one row or one column starts from
//!   [`Composition::grid`] and places views with
//!   [`with_cell`](Composition::with_cell) / [`with_spanning_cell`].
//! * **A spanning cell is how a wide header shares tracks with two narrow
//!   views below it.** `with_spanning_cell(0, 0, 1, 2, bar)` is "one row tall, both
//!   columns wide"; the two bottom cells keep their own columns.
//! * **`Align::Each`, not `All`, because the tracks are deliberately unequal.** A
//!   dashboard of *different* questions wants panels of different sizes, and `Each`
//!   shares a rectangle only inside a track — so the header spans, the histogram
//!   gets two thirds of the width and the scatter one third. `Align::All` would
//!   instead give every panel one size (see `composition_same_panels.rs`), which is
//!   the opposite of what an unequal grid is for.
//! * **`Resolve::Independent` when the cells plot different quantities.** The three
//!   charts count different things, so they share a rectangle but not a scale;
//!   `Resolve::Shared` would force all of them onto the count axis.
//! * **Per-axis gaps.** `with_row_spacing` is small (a header belongs to the row
//!   below it) while `with_column_spacing` is wider (two independent charts).
//! * **One legend for the page**, hoisted out of the scatter's cell, and named with
//!   `with_legend_from` because two cells have three traces each.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    // Row 0, both columns: mean sepal width per species.
    let bars = chart!(&iris)?
        .mark_bar()?
        .encode((
            alt::x("species"),
            alt::y("sepal_width").with_aggregate("mean"),
            alt::color("species"),
        ))?
        .with_y_label("mean sepal width")
        .with_show_legend(false);

    // Row 1, column 0: how petal length is distributed.
    let histogram = chart!(&iris)?
        .mark_hist()?
        .encode((alt::x("petal_length").with_bins(20), alt::y("count")))?
        .with_show_legend(false);

    // Row 1, column 1: the relation between the two.
    let scatter = chart!(&iris)?
        .mark_point()?
        .encode((
            alt::x("petal_length"),
            alt::y("petal_width"),
            alt::color("species"),
        ))?
        .configure_point(|point| point.with_size(5.0).with_opacity(0.85));

    Composition::grid(2, 2)
        // A header is fixed and the two panels below take the rest. Two thirds of
        // the width for the histogram, one third for the scatter.
        .with_row_height(0, TrackSize::fixed(200.0))
        .with_row_height(1, TrackSize::fill(1.0))
        .with_column_width(0, TrackSize::fill(2.0))
        .with_column_width(1, TrackSize::fill(1.0))
        // The header spans both columns; (row, col, row_span, col_span, view).
        .with_spanning_cell(0, 0, 1, 2, bars)
        .with_cell(1, 0, histogram)
        .with_cell(1, 1, scatter)
        // Panels align inside their track; every chart keeps its own scale.
        .with_align(Align::Each)
        .with_resolve(Resolve::Independent)
        .with_row_spacing(8.0)
        .with_column_spacing(16.0)
        // The legend belongs to the scatter; say so, because the bar chart has a
        // colour scale too.
        .with_legend_position(LegendPosition::Right)
        .with_legend_from(1, 1)
        .with_title("Iris: one table, three questions")
        .with_description("Mean sepal width by species, petal length distribution, and petal length against petal width.")
        .with_size(760, 560)
        .save("docs/src/images/composition_dashboard.svg")?;

    println!("Saved docs/src/images/composition_dashboard.svg");
    Ok(())
}
