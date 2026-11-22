//! Marginal histograms around a binned heatmap — Vega-Lite's recipe, on iris.
//!
//! This is Vega-Lite's [marginal histograms example](https://vega.github.io/vega-lite/examples/concat_marginal_histograms.html)
//! rebuilt with charton's composition layer. The JSON says
//!
//! ```text
//! vconcat [
//!   bar    {height: 60, x: bin(field, axis: null), y: count()},
//!   hconcat [
//!     rect {x: bin(a), y: bin(b), color: count()},
//!     bar  {width: 60, y: bin(field, axis: null), x: count()}
//!   ]
//! ]
//! ```
//!
//! and each of those clauses has a name here:
//!
//! | Vega-Lite | charton |
//! | --- | --- |
//! | `vconcat` / `hconcat` | one grid, cells placed with `with_cell(row, col, view)` |
//! | `"height": 60` / `"width": 60` | `with_row_height(0, TrackSize::fixed(60.0))` / `with_column_width(1, …)` |
//! | `"axis": null` | `with_show_x_axis(false)` / `with_show_y_axis(false)` |
//! | `"bounds": "flush"` + `"align": "each"` | `with_align(Align::Each)` |
//! | `"spacing": 15` | `with_spacing(6.0)` |
//! | `"domain": [0, 1000]` | `with_y_domain(0.0, max)` on both strips — optional, see below |
//!
//! The page is the same 2 × 2 grid as the scatter example, with one intersection
//! left empty:
//!
//! ```text
//!              column 0                  column 1
//!            +-------------------------+-----------+
//!   row 0    | (0, 0) top histogram    |  (empty)  |   60px
//!            +-------------------------+-----------+
//!   row 1    | (1, 0) count heatmap    | (1, 1)    |   the rest
//!            |                         | right hist|
//!            +-------------------------+-----------+
//!                 the rest                60px
//! ```
//!
//! Two choices worth knowing:
//!
//! * **The count scale is not pinned.** Vega-Lite pins one domain (`[0, 1000]`) for
//!   both strips so that a height means the same count in both. That is one
//!   `with_y_domain(0.0, max)` per strip; leaving it out lets each strip use its
//!   whole height for its own fullest bin, which is what `ggExtra` does and what
//!   the scatter example does.
//! * **The legend is drawn by the page**, on its right edge
//!   (`with_legend_position`). It belongs to the heatmap, and its own right margin
//!   falls between the heatmap and the right-hand strip.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;
    let bins = 18;

    // The main cell: how many observations fall in each
    // (petal_length, sepal_width) bin.
    let heatmap = chart!(&iris)?
        .mark_rect()?
        .encode((
            alt::x("petal_length").with_bins(bins),
            alt::y("sepal_width").with_bins(bins),
            alt::color("petal_length").with_aggregate("count"),
        ))?
        .with_color_label("Count of Records");

    // The same counts, one axis at a time.
    let top_hist = chart!(&iris)?
        .mark_hist()?
        .encode((alt::x("petal_length").with_bins(bins), alt::y("count")))?
        .with_y_label("")
        .with_show_x_axis(false)
        .with_grid(false);

    let right_hist = chart!(&iris)?
        .mark_hist()?
        .encode((alt::x("sepal_width").with_bins(bins), alt::y("count")))?
        // The count is the y *channel* even though `coord_flip` draws it along the
        // bottom: a domain follows the channel, an axis follows the edge.
        .coord_flip()
        .with_y_label("")
        .with_show_y_axis(false)
        .with_grid(false);

    Composition::grid(2, 2)
        .with_column_width(0, TrackSize::fill(1.0))
        .with_column_width(1, TrackSize::fixed(60.0))
        .with_row_height(0, TrackSize::fixed(60.0))
        .with_row_height(1, TrackSize::fill(1.0))
        .with_cell(0, 0, top_hist)
        .with_cell(1, 0, heatmap)
        // (0, 1) stays empty: the blank corner above the right-hand strip.
        .with_cell(1, 1, right_hist)
        .with_align(Align::Each)
        .with_spacing(6.0)
        .with_legend_position(LegendPosition::Right)
        .save("docs/src/images/concat_marginal_histograms.svg")?;

    println!("Saved docs/src/images/concat_marginal_histograms.svg");
    Ok(())
}
