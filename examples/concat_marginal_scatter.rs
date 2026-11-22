//! A scatter with marginal histograms — the `ggExtra::ggMarginal` recipe, on iris.
//!
//! The R version this reproduces:
//!
//! ```r
//! p1 <- penguins %>%
//!   ggplot(aes(x = culmen_length_mm, y = body_mass_g, color = species)) +
//!   geom_point() +
//!   theme(legend.position = "left")
//!
//! ggMarginal(p1, type = "histogram", groupColour = TRUE, groupFill = TRUE)
//! ```
//!
//! Three charts, one marginal page:
//!
//! ```text
//!              column 0                  column 1
//!            +-------------------------+-----------+
//!   row 0    | (0, 0) top histogram    |  (empty)  |   60px
//!            +-------------------------+-----------+
//!   row 1    | (1, 0) scatter          | (1, 1)    |   the rest
//!            |                         | right hist|
//!            +-------------------------+-----------+
//!                 the rest                60px
//! ```
//!
//! That grid is what `Composition::marginal` builds, so the whole layout is one
//! call. The top-right intersection holds no cell on purpose: the strip stops at
//! the scatter's right edge instead of running over the right-hand strip, exactly
//! as in the R figure.
//!
//! What makes it a *marginal* plot rather than three charts side by side:
//!
//! * `Align::Each` (which `Composition::marginal` sets for you) — the strips
//!   line up with the scatter's **data rectangle**,
//!   not with its canvas, and the cells of a column also share a scale. That is
//!   why the bins sit exactly above (and beside) the points they count.
//! * `with_show_x_axis(false)` / `with_show_y_axis(false)` — a strip draws no axis
//!   of the variable it shares; the scatter's axis already carries that scale, and
//!   repeating its ticks would draw the same numbers twice.
//! * `with_legend_position(LegendPosition::Right)` — the legend is declared by the
//!   scatter, but the page draws it, on the outer edge. Left inside its own cell
//!   it would land between the scatter and the right-hand strip.
//!
//! Each strip is scaled to its own fullest bin, as `ggExtra` does. The Vega-Lite
//! variant in `concat_marginal_histograms.rs` makes the same choice and shows the
//! one line that pins a common count scale instead.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;
    let bins = 18;

    let scatter: LayeredChart = chart!(&iris)?
        .mark_point()?
        .encode((
            alt::x("petal_length"),
            alt::y("sepal_width"),
            alt::color("species"),
        ))?
        .configure_point(|point| point.with_size(4.0).with_opacity(0.85))
        .into();

    // `groupColour = TRUE, groupFill = TRUE`: one histogram per species, drawn
    // from the same baseline with alpha, so they overlap instead of stacking.
    //
    // A local helper, not part of the API: given a column it returns the strip
    // that summarises that column. The two strips differ only in that column.
    let marginal_hist = |field: &str| -> Result<LayeredChart, Box<dyn Error>> {
        Ok(chart!(&iris)?
            .mark_hist()?
            .configure_hist(|hist| hist.with_opacity(0.55))
            .encode((
                alt::x(field).with_bins(bins),
                alt::y("count"),
                alt::color("species"),
            ))?
            .with_show_x_axis(false)
            .with_show_y_axis(false)
            .with_grid(false)
            .with_show_legend(false))
    };

    Composition::marginal(
        scatter,
        Marginals::new()
            // Summarises the x axis: the top strip shares the scatter's column.
            .with_top(marginal_hist("petal_length")?)
            // Summarises the y axis, so it is flipped to run vertically.
            .with_right(marginal_hist("sepal_width")?.coord_flip())
            .with_strip_size(60.0),
    )
    .with_spacing(6.0)
    .with_legend_position(LegendPosition::Right)
    .save("docs/src/images/concat_marginal_scatter.svg")?;

    println!("Saved docs/src/images/concat_marginal_scatter.svg");
    Ok(())
}
