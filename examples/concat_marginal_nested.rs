//! A marginal plot built by **nesting** — the Vega-Lite shape of the same figure.
//!
//! The two other marginal recipes build one flat grid, because that is the
//! shortest path to the picture. This one builds the figure the way Vega-Lite's
//! own spec writes it:
//!
//! ```text
//! vconcat[
//!   bar    {height: 60, x: bin(a), y: count()},          // top strip
//!   hconcat[
//!     rect {x: bin(a), y: bin(b), color: count()},       // main view
//!     bar  {width: 60, y: bin(b), x: count()}            // right strip
//!   ]
//! ]
//! ```
//!
//! and that is the same page:
//!
//! ```text
//!   Composition::vconcat((
//!       top_strip,
//!       Composition::hconcat((main, right_strip))   // <- a page inside a page
//!   ))
//! ```
//!
//! # How a page inside a page aligns
//!
//! The strip at the top and the main view live in **different** compositions: the
//! strip is a cell of the outer page, the main view is a cell of the inner one. A
//! nested page is one cell to its parent, so it answers with the rectangle of its
//! **master cell** — its largest, which here is the heatmap. That is what the outer
//! page aligns the strip with, and pins.
//!
//! Three details make the nesting behave:
//!
//! * **the inner page has no margins of its own.** Margins belong to the page you
//!   are looking at; an inner page that adds its own would shift its cells away
//!   from the outer page's data rectangle. This is what
//!   `configure_theme(…margin(0.0))` is doing below.
//! * **both pages are `Align::Each`.** The outer aligns the strip with the
//!   inner page's *master panel* (its largest cell), the inner one aligns its two
//!   cells with each other.
//! * **the inner page reports its master cell's scale**, so the outer page can
//!   hand the top strip the main view's x domain instead of its own.
//!
//! Compare `concat_marginal_histograms.rs`, which draws the same figure as one
//! flat 2 × 2 grid: fewer moving parts, and the one to reach for unless the
//! nesting is what you want to express.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;
    let bins = 18;

    // The main view: how many observations fall in each (petal_length,
    // sepal_width) bin.
    let heatmap = chart!(&iris)?
        .mark_rect()?
        .encode((
            alt::x("petal_length").with_bins(bins),
            alt::y("sepal_width").with_bins(bins),
            alt::color("petal_length").with_aggregate("count"),
        ))?
        .with_color_label("Count of Records");

    // A strip: one histogram, summarising one column, drawn without the axis it
    // shares and without a legend of its own.
    let strip = |field: &str| -> Result<LayeredChart, Box<dyn Error>> {
        Ok(chart!(&iris)?
            .mark_hist()?
            .encode((alt::x(field).with_bins(bins), alt::y("count")))?
            .with_show_x_axis(false)
            .with_show_y_axis(false)
            .with_grid(false)
            .with_show_legend(false))
    };

    // The inner page: the main view and the strip that summarises its y axis.
    // `coord_flip` turns that histogram to run vertically, so it borrows the
    // main view's *vertical* scale.
    let inner = Composition::hconcat((heatmap, strip("sepal_width")?.coord_flip()))
        .with_column_width(0, TrackSize::fill(1.0))
        .with_column_width(1, TrackSize::fixed(60.0))
        .with_align(Align::Each)
        // An inner page carries no margins: they belong to the page it sits in.
        .configure_theme(|theme| {
            theme
                .with_top_margin(0.0)
                .with_bottom_margin(0.0)
                .with_left_margin(0.0)
                .with_right_margin(0.0)
        });

    // The outer page: the strip that summarises x, above that inner page. It is a
    // tuple, not an array, because the two views are different types.
    Composition::vconcat((strip("petal_length")?, inner))
        .with_row_height(0, TrackSize::fixed(60.0))
        .with_row_height(1, TrackSize::fill(1.0))
        .with_align(Align::Each)
        .with_spacing(6.0)
        .with_legend_position(LegendPosition::Right)
        .save("docs/src/images/concat_marginal_nested.svg")?;

    println!("Saved docs/src/images/concat_marginal_nested.svg");
    Ok(())
}
