//! Composition: several views on one page.
//!
//! These tests read the exported document, because the promise of a composition
//! is a geometric one: the cells must land where the layout says, and a flush
//! cell must share the *same* data rectangle (and therefore the same
//! value-to-pixel mapping) as its neighbours. Asserting on numbers taken from the
//! SVG is the only way to check that without trusting the implementation.

mod common;

use charton::prelude::*;
use common::{attr, groups_by_class, plot_rect};
use std::error::Error;

/// Eleven points along a line, so a value maps to one unambiguous pixel.
///
/// The range is 0..=10 on purpose: with five bins the midpoints are 1, 3, 5, 7
/// and 9, so a histogram bar and a scatter point stand for the very same value.
fn sample() -> Result<LayeredChart, Box<dyn Error>> {
    let x: Vec<f64> = (0..=10).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| 10.0 + v * 2.0).collect();
    Ok(chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .into())
}

/// The histogram of the same x column, as a strip without axis furniture.
fn strip() -> Result<LayeredChart, Box<dyn Error>> {
    let x: Vec<f64> = (0..=10).map(|i| i as f64).collect();
    let ds = Dataset::new().with_column("x", x)?;
    Ok(chart!(&ds)?
        .mark_hist()?
        .encode((alt::x("x").with_bins(5), alt::y("count")))?
        .with_show_x_axis(false)
        .with_show_y_axis(false)
        .with_grid(false))
}

/// A column of `[strip, main]`: the classic marginal layout.
fn stacked(align: Align) -> Result<Composition, Box<dyn Error>> {
    Ok(Composition::grid(2, 1)
        .with_row_height(0, TrackSize::fixed(70.0))
        .with_row_height(1, TrackSize::fill(1.0))
        .with_cell(0, 0, strip()?)
        .with_cell(1, 0, sample()?)
        .with_align(align)
        .with_size(600, 460))
}

/// Every `<circle>` centre in a cell, as `(cx, cy)`.
fn circles(cell: &str) -> Vec<(f64, f64)> {
    cell.lines()
        .filter(|line| line.starts_with("<circle"))
        .map(|line| (attr(line, "cx"), attr(line, "cy")))
        .collect()
}

/// The centres of every histogram bar in a cell, left to right.
///
/// A cell holds no background rectangle of its own (the page paints one), so
/// every `<rect>` inside it is a bar.
fn bar_centres(cell: &str) -> Vec<f64> {
    let mut centres: Vec<f64> = cell
        .lines()
        .filter(|line| line.starts_with("<rect"))
        .map(|line| attr(line, "x") + attr(line, "width") / 2.0)
        .collect();
    centres.sort_by(f64::total_cmp);
    centres
}

/// The clip rectangle of the plot scope inside one cell.
///
/// Clip paths live in the document-level `<defs>`, so the cell is followed to the
/// definition it references.
fn cell_panel(svg: &str, cell: &str) -> (f64, f64, f64, f64) {
    let id = cell
        .split("clip-path=\"url(#")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .expect("expected the plot scope to carry a clip reference");
    let marker = format!("<clipPath id=\"{id}\"");
    let line = svg
        .lines()
        .find(|line| line.contains(&marker))
        .unwrap_or_else(|| panic!("no clipPath definition for {id}"));
    (
        attr(line, "x"),
        attr(line, "y"),
        attr(line, "width"),
        attr(line, "height"),
    )
}

#[test]
fn a_flush_strip_shares_the_panel_of_the_view_below_it() -> Result<(), Box<dyn Error>> {
    let svg = stacked(Align::Each)?.to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");
    assert_eq!(cells.len(), 2, "one view scope per cell");

    let (sx, _, sw, _) = cell_panel(&svg, cells[0]);
    let (mx, _, mw, _) = cell_panel(&svg, cells[1]);

    assert!(
        (sx - mx).abs() < 0.5 && (sw - mw).abs() < 0.5,
        "the strip's data rectangle {sx}/{sw} must match the main panel's {mx}/{mw}"
    );
    Ok(())
}

#[test]
fn a_value_lands_on_the_same_x_in_the_strip_and_in_the_chart() -> Result<(), Box<dyn Error>> {
    let svg = stacked(Align::Each)?.to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");

    // The points are at x = 0..=10 and the strip's five bins are centred on
    // 1, 3, 5, 7 and 9. Each bar must therefore sit exactly under the point with
    // the same x. Sharing the panel alone would line the rectangles up while the
    // values still disagreed; this is the check that the scale is shared too.
    let mut points: Vec<f64> = circles(cells[1]).into_iter().map(|(cx, _)| cx).collect();
    points.sort_by(f64::total_cmp);
    assert_eq!(points.len(), 11, "one point per value in 0..=10");

    let centres = bar_centres(cells[0]);
    assert_eq!(centres.len(), 5, "five bins, five bars");

    for (bin, centre) in centres.iter().enumerate() {
        let value = (bin * 2 + 1) as f64;
        let point = points[bin * 2 + 1];
        assert!(
            (centre - point).abs() < 1.0,
            "the bin centred on x = {value} is drawn at {centre}, but the point at x = {value}              is at {point}"
        );
    }
    Ok(())
}

#[test]
fn full_bounds_leave_each_view_where_its_own_layout_put_it() -> Result<(), Box<dyn Error>> {
    let svg = stacked(Align::None)?.to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");
    let (sx, _, sw, _) = cell_panel(&svg, cells[0]);
    let (mx, _, mw, _) = cell_panel(&svg, cells[1]);

    // The strip draws no y axis, so its panel starts further left than the main
    // panel's: under `Full` that difference is kept.
    assert!(
        sx < mx - 1.0 && sw > mw + 1.0,
        "under full bounds the panels must stay independent: strip {sx}/{sw}, main {mx}/{mw}"
    );
    Ok(())
}

#[test]
fn hiding_an_axis_frees_its_band() -> Result<(), Box<dyn Error>> {
    let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    let with_axis = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_size(400, 300)
        .to_svg()?;
    let without = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_show_x_axis(false)
        .with_size(400, 300)
        .to_svg()?;

    let (_, _, _, shown_height) = plot_rect(&with_axis);
    let (_, _, _, hidden_height) = plot_rect(&without);
    assert!(
        hidden_height > shown_height + 20.0,
        "a hidden x axis must release its band: {shown_height} -> {hidden_height}"
    );
    Ok(())
}

/// A composition adds nothing of its own: with the page margins zeroed, one cell
/// holding one chart draws exactly that chart.
///
/// (With the default margins it draws that chart *inset by them*, which is what a
/// page does. The margins are the only thing the page contributes, so they are the
/// only thing this test has to take out to compare.)
#[test]
fn a_one_cell_composition_draws_the_chart_unchanged() -> Result<(), Box<dyn Error>> {
    let x = [1.0, 2.0, 3.0];
    let y = [2.0, 4.0, 3.0];
    let chart: LayeredChart = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_size(400, 300);

    // What "the same picture" means: the same primitives at the same coordinates.
    let direct = common::shapes(&chart.to_svg()?).join(
        "
",
    );
    let composed = common::shapes(
        &Composition::grid(1, 1)
            .with_cell(0, 0, chart)
            .with_size(400, 300)
            .configure_theme(|theme| {
                theme
                    .with_top_margin(0.0)
                    .with_bottom_margin(0.0)
                    .with_left_margin(0.0)
                    .with_right_margin(0.0)
            })
            .to_svg()?,
    )
    .join(
        "
",
    );

    assert_eq!(
        direct, composed,
        "one cell, no page margins: the page must draw the chart itself"
    );
    Ok(())
}

#[test]
fn a_composition_reports_its_own_size() -> Result<(), Box<dyn Error>> {
    let svg = stacked(Align::Each)?.with_size(720, 480).to_svg()?;
    let root = svg.lines().next().expect("an svg root element");
    assert_eq!(attr(root, "width"), 720.0);
    assert_eq!(attr(root, "height"), 480.0);
    Ok(())
}

#[test]
fn hconcat_places_views_side_by_side() -> Result<(), Box<dyn Error>> {
    let svg = Composition::hconcat([sample()?, sample()?])
        .with_size(600, 300)
        .to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");
    assert_eq!(cells.len(), 2);

    let (left, _, _, _) = cell_panel(&svg, cells[0]);
    let (right, _, _, _) = cell_panel(&svg, cells[1]);
    assert!(right > left, "the second cell must be to the right");
    Ok(())
}

#[test]
fn a_cell_can_span_two_tracks() -> Result<(), Box<dyn Error>> {
    // A wide strip over two columns: it must be wider than either column.
    let svg = Composition::grid(2, 2)
        .with_row_height(0, TrackSize::fixed(60.0))
        .with_spanning_cell(0, 0, 1, 2, strip()?)
        .with_cell(1, 0, sample()?)
        .with_cell(1, 1, sample()?)
        .with_size(600, 400)
        .to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");
    let (_, _, span_width, _) = cell_panel(&svg, cells[0]);
    let (_, _, column_width, _) = cell_panel(&svg, cells[1]);
    assert!(
        span_width > column_width * 1.5,
        "a spanning cell covers both columns: {span_width} against {column_width}"
    );
    Ok(())
}

#[cfg(feature = "png")]
#[test]
fn a_composition_exports_png_at_the_page_size() -> Result<(), Box<dyn Error>> {
    let png = stacked(Align::Each)?.with_size(320, 240).to_png()?;
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
    assert_eq!((width, height), (320, 240));
    Ok(())
}

/// The vertical twin of the marginal layout: a **flipped** histogram beside the
/// scatter, sharing the row's scale.
///
/// This is the case that proves scale sharing follows the *physical* axis: after
/// `coord_flip` the strip's own y channel is the count, so a channel-based rule
/// would have given the strip the scatter's y domain and squashed its bars.
#[test]
fn a_flipped_strip_shares_the_rows_scale() -> Result<(), Box<dyn Error>> {
    let values: Vec<f64> = (0..=10).map(|i| i as f64).collect();
    let ds = Dataset::new()
        .with_column("x", values.clone())?
        .with_column("y", values.clone())?;

    let scatter: LayeredChart = chart!(&ds)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .into();

    let strip: LayeredChart = chart!(&ds)?
        .mark_hist()?
        .encode((alt::x("y").with_bins(5), alt::y("count")))?
        .coord_flip()
        .with_show_x_axis(false)
        .with_show_y_axis(false)
        .with_grid(false);

    let svg = Composition::grid(1, 2)
        .with_column_width(0, TrackSize::fill(1.0))
        .with_column_width(1, TrackSize::fixed(70.0))
        .with_cell(0, 0, scatter)
        .with_cell(0, 1, strip)
        .with_align(Align::Each)
        .with_size(600, 380)
        .to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");

    // Same row, same data rectangle.
    let (_, sy, _, sh) = cell_panel(&svg, cells[0]);
    let (_, ty, _, th) = cell_panel(&svg, cells[1]);
    assert!(
        (sy - ty).abs() < 0.5 && (sh - th).abs() < 0.5,
        "the right strip's panel ({ty}/{th}) must match the scatter's ({sy}/{sh})"
    );

    // Each bar is centred on 1, 3, 5, 7 and 9; each of those values is also a
    // point, so the centres have to line up.
    let mut points: Vec<f64> = circles(cells[0]).into_iter().map(|(_, cy)| cy).collect();
    points.sort_by(f64::total_cmp);
    assert_eq!(points.len(), 11);

    let mut centres: Vec<f64> = cells[1]
        .lines()
        .filter(|line| line.starts_with("<rect"))
        .map(|line| attr(line, "y") + attr(line, "height") / 2.0)
        .collect();
    centres.sort_by(f64::total_cmp);
    assert_eq!(centres.len(), 5, "five bins, five bars");

    for (bin, centre) in centres.iter().enumerate() {
        // Pixels grow downwards, so the sorted centres run from y = 9 down to
        // y = 1, while the points (one per 0..=10) start with y = 10 at the top.
        let value = 9.0 - (bin * 2) as f64;
        let point = points[bin * 2 + 1];
        assert!(
            (centre - point).abs() < 1.0,
            "the bin centred on y = {value} is drawn at {centre}, but the point at y = {value} \
             is at {point}"
        );
    }

    // The strip keeps its own count scale: the longest bar fills the strip
    // instead of being measured against the scatter's y range.
    let longest = cells[1]
        .lines()
        .filter(|line| line.starts_with("<rect"))
        .map(|line| attr(line, "width"))
        .fold(0.0, f64::max);
    let (_, _, panel_width, _) = cell_panel(&svg, cells[1]);
    assert!(
        longest > panel_width * 0.55,
        "the longest bar ({longest}) should fill the strip ({panel_width})"
    );
    Ok(())
}

/// A view may ask for its own room, the way Vega-Lite's `"height": 60` does.
#[test]
fn a_view_can_size_its_own_track() -> Result<(), Box<dyn Error>> {
    let svg = Composition::vconcat([strip()?.with_height(TrackSize::fixed(80.0)), sample()?])
        .with_spacing(6.0)
        .with_size(600, 460)
        .to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");

    let (_, sy, _, sh) = cell_panel(&svg, cells[0]);
    let (_, my, _, _) = cell_panel(&svg, cells[1]);
    assert!(
        sh < 80.0 && sh > 50.0,
        "the strip asked for an 80px track, so its panel is a little less: {sh}"
    );
    assert!(
        my > sy + sh,
        "the main view starts below the strip ({my} vs {} + {sh})",
        sy
    );
    Ok(())
}

/// An explicit track size beats the size a view asks for.
#[test]
fn an_explicit_track_size_wins() -> Result<(), Box<dyn Error>> {
    let svg = Composition::grid(2, 1)
        .with_row_height(0, TrackSize::fixed(120.0))
        .with_cell(0, 0, strip()?.with_height(TrackSize::fixed(40.0)))
        .with_cell(1, 0, sample()?)
        .with_size(600, 460)
        .to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");
    let (_, _, _, sh) = cell_panel(&svg, cells[0]);
    assert!(
        sh > 90.0,
        "the row is 120px, not the 40px the view asked for: {sh}"
    );
    Ok(())
}

/// The band a legend was drawn into, resolved through its clip path.
fn legend_band(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let line = svg.lines().find(|line| line.contains("charton-legend"))?;
    let id = line
        .split("clip-path=\"url(#")
        .nth(1)
        .and_then(|rest| rest.split(')').next())?;
    let marker = format!("<clipPath id=\"{id}\"");
    let clip = svg.lines().find(|line| line.contains(&marker))?;
    Some((
        attr(clip, "x"),
        attr(clip, "y"),
        attr(clip, "width"),
        attr(clip, "height"),
    ))
}

/// A marginal layout whose legend is hoisted to the page edge.
fn hoisted() -> Result<Composition, Box<dyn Error>> {
    let x: Vec<f64> = (0..=10).map(|i| i as f64).collect();
    let ds = Dataset::new()
        .with_column("x", x.clone())?
        .with_column("y", x.clone())?
        .with_column(
            "group",
            (0..=10).map(|i| format!("g{}", i % 3)).collect::<Vec<_>>(),
        )?;

    let scatter: LayeredChart = chart!(&ds)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y"), alt::color("group")))?
        .into();
    let strip: LayeredChart = chart!(&ds)?
        .mark_hist()?
        .encode((alt::x("x").with_bins(5), alt::y("count")))?
        .with_show_x_axis(false)
        .with_show_y_axis(false)
        .with_grid(false);

    Ok(Composition::grid(2, 1)
        .with_row_height(0, TrackSize::fixed(60.0))
        .with_row_height(1, TrackSize::fill(1.0))
        .with_cell(0, 0, strip)
        .with_cell(1, 0, scatter)
        .with_align(Align::Each)
        .with_size(600, 400))
}

#[test]
fn a_hoisted_legend_sits_outside_every_cell() -> Result<(), Box<dyn Error>> {
    for edge in [LegendPosition::Left, LegendPosition::Right] {
        let svg = hoisted()?.with_legend_position(edge).to_svg()?;
        let cells = groups_by_class(&svg, "charton-view");
        let (lx, _, lw, _) = legend_band(&svg).expect("the page draws a legend");

        let leftmost = cells
            .iter()
            .map(|cell| cell_panel(&svg, cell).0)
            .fold(f64::INFINITY, f64::min);
        let rightmost = cells
            .iter()
            .map(|cell| {
                let (x, _, w, _) = cell_panel(&svg, cell);
                x + w
            })
            .fold(f64::NEG_INFINITY, f64::max);

        match edge {
            LegendPosition::Right => assert!(
                lx >= rightmost,
                "{edge:?}: the legend starts at {lx} but the rightmost panel ends at {rightmost}"
            ),
            _ => assert!(
                lx + lw <= leftmost,
                "{edge:?}: the legend ends at {} but the leftmost panel starts at {leftmost}",
                lx + lw
            ),
        }

        // Nothing draws a legend inside a cell any more.
        for cell in &cells {
            assert!(
                !cell.contains("charton-legend"),
                "{edge:?}: a cell still draws its own legend"
            );
        }
        // ...and the entries survive the move.
        assert_eq!(svg.matches("charton-legend-entry").count(), 3, "{edge:?}");
    }
    Ok(())
}

#[test]
fn hoisting_frees_the_side_the_legend_used_to_occupy() -> Result<(), Box<dyn Error>> {
    let in_cell = hoisted()?.to_svg()?;
    let in_cell_cells = groups_by_class(&in_cell, "charton-view");
    let (ix, _, iw, _) = cell_panel(&in_cell, in_cell_cells[1]);

    let hoisted_svg = hoisted()?
        .with_legend_position(LegendPosition::Left)
        .to_svg()?;
    let hoisted_cells = groups_by_class(&hoisted_svg, "charton-view");
    let (hx, _, hw, _) = cell_panel(&hoisted_svg, hoisted_cells[1]);

    // In-cell the legend sat in the main cell's right band, so its panel stopped
    // short of the page edge. Hoisted, the band is gone from the cell and the
    // panel runs on to the cell's own margin.
    assert!(
        hx + hw > ix + iw + 1.0,
        "the main panel should reach further right: {} -> {}",
        ix + iw,
        hx + hw
    );
    Ok(())
}

#[test]
fn hoisting_without_a_legend_changes_nothing() -> Result<(), Box<dyn Error>> {
    // Same layout, but neither cell declares a guide, so there is nothing to hoist.
    let x: Vec<f64> = (0..=10).map(|i| i as f64).collect();
    let ds = Dataset::new()
        .with_column("x", x.clone())?
        .with_column("y", x)?
        .with_column("v", (0..=10).map(|i| i as f64).collect::<Vec<_>>())?;
    let build = || -> Result<Composition, Box<dyn Error>> {
        let scatter: LayeredChart = chart!(&ds)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y")))?
            .into();
        let mut strip: LayeredChart = chart!(&ds)?
            .mark_hist()?
            .encode((alt::x("x").with_bins(5), alt::y("count")))?
            .into();
        strip = strip
            .with_show_x_axis(false)
            .with_show_y_axis(false)
            .with_grid(false);
        Ok(Composition::grid(2, 1)
            .with_row_height(0, TrackSize::fixed(60.0))
            .with_row_height(1, TrackSize::fill(1.0))
            .with_cell(0, 0, strip)
            .with_cell(1, 0, scatter)
            .with_align(Align::Each)
            .with_size(600, 400))
    };

    let plain = build()?.to_svg()?;
    let asked = build()?
        .with_legend_position(LegendPosition::Left)
        .to_svg()?;
    let plain_cells = groups_by_class(&plain, "charton-view");
    let asked_cells = groups_by_class(&asked, "charton-view");
    assert_eq!(
        cell_panel(&plain, plain_cells[1]),
        cell_panel(&asked, asked_cells[1]),
        "a page with nothing to hoist must not reserve a band"
    );
    assert!(legend_band(&asked).is_none());
    Ok(())
}

/// A 3 × 3 grid: each axis is solved on its own, so the two dimensions never
/// interact. With every track flexible, the page is split into three equal
/// columns and three equal rows (minus the two gaps on each axis).
#[test]
fn a_three_by_three_grid_splits_each_axis_independently() -> Result<(), Box<dyn Error>> {
    // A cell whose panel *is* its cell: no margins, no axes, no grid.
    let bare = || -> Result<LayeredChart, Box<dyn Error>> {
        let x = [1.0, 2.0, 3.0];
        let y = [1.0, 2.0, 3.0];
        Ok(chart!(&x, &y)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y")))?
            .with_margins(0.0, 0.0, 0.0, 0.0)
            .with_show_x_axis(false)
            .with_show_y_axis(false)
            .with_grid(false))
    };

    // Page margins off, so the arithmetic under test is just the track split.
    let mut page = Composition::grid(3, 3)
        .with_spacing(6.0)
        .with_align(Align::Each)
        .configure_theme(|theme| {
            theme
                .with_top_margin(0.0)
                .with_bottom_margin(0.0)
                .with_left_margin(0.0)
                .with_right_margin(0.0)
        })
        .with_size(500, 400);
    for row in 0..3 {
        for col in 0..3 {
            page = page.with_cell(row, col, bare()?);
        }
    }

    let svg = page.to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");
    assert_eq!(cells.len(), 9);

    let panels: Vec<(f64, f64, f64, f64)> =
        cells.iter().map(|cell| cell_panel(&svg, cell)).collect();

    // columns: (500 - 2 gaps) / 3 = 162.67, rows: (400 - 2 gaps) / 3 = 129.33
    for row in 0..3 {
        for col in 0..3 {
            let (x, y, w, h) = panels[row * 3 + col];
            let expected_x = col as f64 * (162.666_666 + 6.0);
            let expected_y = row as f64 * (129.333_333 + 6.0);
            assert!(
                (x - expected_x).abs() < 0.6
                    && (y - expected_y).abs() < 0.6
                    && (w - 162.666_666).abs() < 0.6
                    && (h - 129.333_333).abs() < 0.6,
                "cell ({row}, {col}) is at {x}/{y} sized {w}x{h}, \
                 expected {expected_x}/{expected_y} sized 162.67x129.33"
            );
        }
    }
    Ok(())
}

/// `Align::All` shares one panel across the **whole page**, not one per track.
///
/// Two columns whose views carry different axis furniture: per-track alignment
/// must leave both panels where their own layout put them (the strip has no y
/// axis, so it starts further left), page-wide alignment must pull them onto one
/// rectangle.
#[test]
fn align_all_gives_every_cell_the_same_panel() -> Result<(), Box<dyn Error>> {
    let page = |align: Align| -> Result<Composition, Box<dyn Error>> {
        Ok(Composition::grid(1, 2)
            .with_cell(0, 0, sample()?)
            .with_cell(0, 1, strip()?)
            .with_align(Align::Each)
            .with_align(align)
            .with_size(600, 460))
    };

    let svg = page(Align::Each)?.to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");
    let left = cell_panel(&svg, cells[0]);
    let right = cell_panel(&svg, cells[1]);
    assert!(
        (left.0 - right.0).abs() > 1.0 && (left.2 - right.2).abs() > 1.0,
        "two columns keep their own panels under Align::Each: {left:?} vs {right:?}"
    );

    let svg = page(Align::All)?.to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");
    let left = cell_panel(&svg, cells[0]);
    let right = cell_panel(&svg, cells[1]);
    // The two panels sit in different columns, so their absolute x differs by the
    // column offset; y, width and height must now agree, which is what makes the
    // two panels one size on one pair of lines.
    for (what, a, b) in [
        ("top", left.1, right.1),
        ("width", left.2, right.2),
        ("height", left.3, right.3),
    ] {
        assert!(
            (a - b).abs() < 0.5,
            "Align::All must give both cells one panel: {what} {a} vs {b} ({left:?} vs {right:?})"
        );
    }
    Ok(())
}

/// A marginal plot written the way Vega-Lite writes it: a strip *above* an
/// `hconcat`, i.e. with the alignment crossing a nesting boundary.
///
/// ```text
/// vconcat[ top strip, hconcat[ scatter, right strip ] ]
/// ```
///
/// Under the old `View` enum a nested composition reported no panel, so flush
/// bounds silently degraded to `Full` for it and the top strip lined up with the
/// inner page's canvas instead of the scatter's data. This is the test that the
/// alignment now crosses that boundary: the bar centres of the top strip must
/// land on the same pixels as the points of the scatter inside the inner page.
#[test]
fn a_nested_marginal_aligns_across_the_nesting() -> Result<(), Box<dyn Error>> {
    // The inner page carries no margins of its own: it is a row inside another
    // page, not a page in its own right.
    let row = Composition::hconcat((sample()?, strip()?))
        .with_column_width(0, TrackSize::fill(1.0))
        .with_column_width(1, TrackSize::fixed(60.0))
        .with_align(Align::Each)
        .configure_theme(|theme| {
            theme
                .with_top_margin(0.0)
                .with_bottom_margin(0.0)
                .with_left_margin(0.0)
                .with_right_margin(0.0)
        });

    let svg = Composition::vconcat((strip()?, row))
        .with_row_height(0, TrackSize::fixed(60.0))
        .with_row_height(1, TrackSize::fill(1.0))
        .with_align(Align::Each)
        .with_spacing(6.0)
        .with_size(600, 460)
        .to_svg()?;

    // The outer page holds the strip and the inner page; the inner page holds the
    // scatter and the right strip. (A cell scope contains its whole subtree, so
    // the inner cells are found inside the outer one.)
    let outer = groups_by_class(&svg, "charton-view");
    assert_eq!(outer.len(), 2, "strip + nested page");
    let inner = groups_by_class(outer[1], "charton-view");
    assert_eq!(inner.len(), 2, "scatter + right strip");

    let mut points: Vec<f64> = circles(inner[0]).into_iter().map(|(cx, _)| cx).collect();
    points.sort_by(f64::total_cmp);
    assert_eq!(points.len(), 11);

    let centres = bar_centres(outer[0]);
    assert_eq!(centres.len(), 5);
    for (bin, centre) in centres.iter().enumerate() {
        let point = points[bin * 2 + 1];
        assert!(
            (centre - point).abs() < 1.0,
            "bin {bin} is drawn at {centre} but its value is at {point}: the nested \
             strip did not adopt the scatter's data rectangle"
        );
    }
    Ok(())
}

/// `Composition::marginal` is the hand-written grid, named — and nothing else.
///
/// The convenience constructor decides only the layout (tracks, sizes, cells,
/// flush bounds). If it were a hidden second engine, the value-to-pixel mapping
/// here would differ from the explicit 2 × 2 grid it replaces.
#[test]
fn marginal_builds_the_same_page_as_the_hand_written_grid() -> Result<(), Box<dyn Error>> {
    let by_hand = Composition::grid(2, 2)
        .with_column_width(0, TrackSize::fill(1.0))
        .with_column_width(1, TrackSize::fixed(60.0))
        .with_row_height(0, TrackSize::fixed(60.0))
        .with_row_height(1, TrackSize::fill(1.0))
        .with_cell(0, 0, strip()?)
        .with_cell(1, 0, sample()?)
        .with_cell(1, 1, strip()?)
        .with_align(Align::Each)
        .with_spacing(6.0)
        .with_size(600, 460);

    let named = Composition::marginal(
        sample()?,
        Marginals::new()
            .with_top(strip()?)
            .with_right(strip()?)
            .with_strip_size(60.0),
    )
    .with_spacing(6.0)
    .with_size(600, 460);

    let by_hand_svg = by_hand.to_svg()?;
    let named_svg = named.to_svg()?;

    let a = groups_by_class(&by_hand_svg, "charton-view");
    let b = groups_by_class(&named_svg, "charton-view");
    assert_eq!(a.len(), 3);
    assert_eq!(b.len(), 3);

    // Same panels, in the same order (main first, then the strips).
    for (hand, named) in a.iter().zip(b.iter()) {
        let hand_panel = cell_panel(&by_hand_svg, hand);
        let named_panel = cell_panel(&named_svg, named);
        for (what, left, right) in [
            ("x", hand_panel.0, named_panel.0),
            ("y", hand_panel.1, named_panel.1),
            ("width", hand_panel.2, named_panel.2),
            ("height", hand_panel.3, named_panel.3),
        ] {
            assert!(
                (left - right).abs() < 0.5,
                "the named marginal moved the {what}: {left} vs {right}"
            );
        }
    }
    Ok(())
}

/// All four edges, and the corners stay empty: a marginal is a grid whose corner
/// intersections hold no cell, not a grid of four strips plus a main view.
#[test]
fn marginal_places_a_strip_on_every_edge_it_is_given() -> Result<(), Box<dyn Error>> {
    let page = Composition::marginal(
        sample()?,
        Marginals::new()
            .with_top(strip()?)
            .with_bottom(strip()?)
            .with_left(strip()?)
            .with_right(strip()?)
            .with_strip_size(40.0),
    )
    .with_spacing(0.0)
    .with_size(600, 460);

    let svg = page.to_svg()?;
    let cells = groups_by_class(&svg, "charton-view");
    assert_eq!(cells.len(), 5, "four strips and the main view");

    let panels: Vec<(f64, f64, f64, f64)> =
        cells.iter().map(|cell| cell_panel(&svg, cell)).collect();
    // Reading order: top, left, main, right, bottom. The main view is the one
    // with the most data room.
    let main_index = (0..panels.len())
        .max_by(|a, b| (panels[*a].2 * panels[*a].3).total_cmp(&(panels[*b].2 * panels[*b].3)))
        .expect("five panels");
    assert_eq!(
        main_index, 2,
        "the main view is the third cell, reading order"
    );
    let main = panels[main_index];
    let (top, left, right, bottom) = (panels[0], panels[1], panels[3], panels[4]);

    // Each strip sits on its own edge of the main view...
    assert!(
        top.1 + top.3 <= main.1 + 0.5,
        "the top strip is above the main view"
    );
    assert!(
        bottom.1 >= main.1 + main.3 - 0.5,
        "the bottom strip is below it"
    );
    assert!(
        left.0 + left.2 <= main.0 + 0.5,
        "the left strip is to its left"
    );
    assert!(
        right.0 >= main.0 + main.2 - 0.5,
        "the right strip is to its right"
    );

    // ...and each pair shares the data rectangle of the axis it summarises: a
    // top/bottom strip shares the main view's x, a left/right strip its y.
    for (what, strip) in [("top", top), ("bottom", bottom)] {
        assert!(
            (strip.0 - main.0).abs() < 0.5 && (strip.2 - main.2).abs() < 0.5,
            "the {what} strip must span the main panel's x: {strip:?} vs {main:?}"
        );
    }
    for (what, strip) in [("left", left), ("right", right)] {
        assert!(
            (strip.1 - main.1).abs() < 0.5 && (strip.3 - main.3).abs() < 0.5,
            "the {what} strip must span the main panel's y: {strip:?} vs {main:?}"
        );
    }
    Ok(())
}

/// Naming the legend owner is a promise, not a hint.
#[test]
fn an_explicit_legend_owner_wins_over_the_largest_cell() -> Result<(), Box<dyn Error>> {
    let x: Vec<f64> = (0..=10).map(|i| i as f64).collect();
    let ds = Dataset::new()
        .with_column("x", x.clone())?
        .with_column("y", x.clone())?
        .with_column(
            "group",
            (0..=10).map(|i| format!("g{}", i % 3)).collect::<Vec<_>>(),
        )?;

    // Two views that each declare a legend; the first is the large one.
    let points = || -> Result<LayeredChart, Box<dyn Error>> {
        Ok(chart!(&ds)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y"), alt::color("group")))?
            .into())
    };

    let page = |owner: Option<(usize, usize)>| -> Result<String, Box<dyn Error>> {
        let mut page = Composition::grid(2, 1)
            .with_row_height(0, TrackSize::fill(3.0))
            .with_row_height(1, TrackSize::fill(1.0))
            .with_cell(0, 0, points()?)
            .with_cell(1, 0, points()?)
            .with_align(Align::Each)
            .with_legend_position(LegendPosition::Right);
        if let Some((row, col)) = owner {
            page = page.with_legend_from(row, col);
        }
        Ok(page.to_svg()?)
    };

    // By default the large cell keeps its band and the small one hands its legend
    // over; naming the small cell reverses exactly that.
    let largest = page(None)?;
    let named = page(Some((1, 0)))?;
    assert_ne!(
        groups_by_class(&largest, "charton-view")[0],
        groups_by_class(&named, "charton-view")[0],
        "naming the other cell must change which cell gives up its legend band"
    );
    Ok(())
}

/// When two cells tie for "largest", the first one is the page's master.
///
/// This is the ambiguous case, and it is worth pinning down: an axis-less cell can
/// have a *bigger panel* than the view it annotates, so choosing the master by
/// panel area would hand the anchor to whichever chart drew less furniture. The
/// master is the largest **canvas**, ties to the first cell added — which here is
/// the scatter, so the strip above still lines up with the points.
#[test]
fn a_tied_master_is_the_first_cell() -> Result<(), Box<dyn Error>> {
    let row = Composition::hconcat((sample()?, strip()?))
        .with_align(Align::Each)
        .configure_theme(|t| {
            t.with_top_margin(0.0)
                .with_bottom_margin(0.0)
                .with_left_margin(0.0)
                .with_right_margin(0.0)
        });
    let svg = Composition::vconcat((strip()?, row))
        .with_row_height(0, TrackSize::fixed(60.0))
        .with_row_height(1, TrackSize::fill(1.0))
        .with_align(Align::Each)
        .with_spacing(6.0)
        .with_size(600, 460)
        .to_svg()?;
    let outer = groups_by_class(&svg, "charton-view");
    let inner = groups_by_class(outer[1], "charton-view");
    let top = cell_panel(&svg, outer[0]);
    let main = cell_panel(&svg, inner[0]);
    assert!(
        (top.0 - main.0).abs() < 0.5 && (top.2 - main.2).abs() < 0.5,
        "the strip above must share the first cell's panel: {top:?} vs {main:?}"
    );
    Ok(())
}

/// `Resolve::Independent` keeps the panels lined up and lets the ranges disagree.
///
/// The two cells sit in one column with different data — the scatter spans 0..=10,
/// the strip 0..=5 — so a shared domain and an independent one put the same bar in
/// different places. Sharing is the default; this is the test that turning it off
/// actually changes what is drawn, and that the panel stays shared either way.
#[test]
fn independent_resolve_keeps_the_panel_but_drops_the_scale() -> Result<(), Box<dyn Error>> {
    let narrow = || -> Result<LayeredChart, Box<dyn Error>> {
        let x: Vec<f64> = (0..=5).map(|i| i as f64).collect();
        let ds = Dataset::new().with_column("x", x)?;
        Ok(chart!(&ds)?
            .mark_hist()?
            .encode((alt::x("x").with_bins(5), alt::y("count")))?
            .with_show_x_axis(false)
            .with_show_y_axis(false)
            .with_grid(false))
    };

    let page = |resolve: Resolve| -> Result<String, Box<dyn Error>> {
        Ok(Composition::grid(2, 1)
            .with_row_height(0, TrackSize::fixed(70.0))
            .with_row_height(1, TrackSize::fill(1.0))
            .with_cell(0, 0, narrow()?)
            .with_cell(1, 0, sample()?)
            .with_align(Align::Each)
            .with_resolve(resolve)
            .with_size(600, 460)
            .to_svg()?)
    };

    let shared = page(Resolve::Shared)?;
    let independent = page(Resolve::Independent)?;

    let shared_cells = groups_by_class(&shared, "charton-view");
    let independent_cells = groups_by_class(&independent, "charton-view");
    assert_eq!(
        cell_panel(&shared, shared_cells[0]),
        cell_panel(&independent, independent_cells[0]),
        "the resolve setting must not move the panel"
    );

    // The strip's five bins are centred on 0.5 ..= 4.5. Sharing the scatter's
    // 0..=10 domain draws them across the left half of the panel; its own 0..=5
    // domain spreads them over the whole of it.
    let shared_bars = bar_centres(shared_cells[0]);
    let independent_bars = bar_centres(independent_cells[0]);
    assert_eq!(shared_bars.len(), 5);
    assert_eq!(independent_bars.len(), 5);
    assert!(
        independent_bars[4] - independent_bars[0] > shared_bars[4] - shared_bars[0] + 1.0,
        "independent bars {independent_bars:?} must span more of the panel than \
         shared ones {shared_bars:?}"
    );

    // ...and under the shared resolve a bin lands on the point of the same value.
    let mut points: Vec<f64> = circles(shared_cells[1])
        .into_iter()
        .map(|(cx, _)| cx)
        .collect();
    points.sort_by(f64::total_cmp);
    for (bin, centre) in shared_bars.iter().enumerate() {
        // The points sit on the integers, so a bin centre of `bin + 0.5` is
        // halfway between two of them.
        let midpoint = (points[bin] + points[bin + 1]) / 2.0;
        assert!(
            (centre - midpoint).abs() < 1.0,
            "the bin centred on {} is at {centre} but the value maps to {midpoint}",
            bin as f64 + 0.5
        );
    }
    Ok(())
}

/// The two axes can have different gaps.
#[test]
fn column_and_row_spacing_are_independent() -> Result<(), Box<dyn Error>> {
    let bare = || -> Result<LayeredChart, Box<dyn Error>> {
        let ds = Dataset::new()
            .with_column("x", vec![1.0, 2.0, 3.0])?
            .with_column("y", vec![1.0, 2.0, 3.0])?;
        Ok(chart!(&ds)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y")))?
            .with_margins(0.0, 0.0, 0.0, 0.0)
            .with_show_x_axis(false)
            .with_show_y_axis(false)
            .with_grid(false))
    };

    let svg = Composition::grid(2, 2)
        .with_column_spacing(40.0)
        .with_row_spacing(5.0)
        .with_cell(0, 0, bare()?)
        .with_cell(0, 1, bare()?)
        .with_cell(1, 0, bare()?)
        .with_cell(1, 1, bare()?)
        .with_size(500, 400)
        .to_svg()?;

    let cells = groups_by_class(&svg, "charton-view");
    assert_eq!(cells.len(), 4);
    let panels: Vec<(f64, f64, f64, f64)> =
        cells.iter().map(|cell| cell_panel(&svg, cell)).collect();
    let (a, b, c) = (panels[0], panels[1], panels[2]);

    assert!(
        (b.0 - (a.0 + a.2) - 40.0).abs() < 1.0,
        "the horizontal gap is {} but was asked for 40",
        b.0 - (a.0 + a.2)
    );
    assert!(
        (c.1 - (a.1 + a.3) - 5.0).abs() < 1.0,
        "the vertical gap is {} but was asked for 5",
        c.1 - (a.1 + a.3)
    );
    Ok(())
}

/// A page has **one** legend position, and two ways to spell it.
///
/// `with_legend_position(edge)` and `configure_theme(|t| t.with_legend_position(edge))`
/// are the same field, so they must produce the same picture — and one of them must
/// not be a silent no-op (`Theme` carries a legend position too, but a page never
/// read it before).
#[test]
fn the_page_legend_position_is_one_setting() -> Result<(), Box<dyn Error>> {
    let direct = hoisted()?
        .with_legend_position(LegendPosition::Left)
        .to_svg()?;
    let through_theme = hoisted()?
        .configure_theme(|theme| theme.with_legend_position(LegendPosition::Left))
        .to_svg()?;
    assert_eq!(direct, through_theme, "the two spellings must agree");

    // ...and the default leaves every legend inside its own cell.
    let untouched = hoisted()?.to_svg()?;
    assert!(
        legend_band(&untouched).is_some(),
        "by default the chart keeps its own legend band"
    );
    assert!(
        groups_by_class(&untouched, "charton-view")[1].contains("charton-legend"),
        "the default draws the legend in the cell, not on the page edge"
    );
    Ok(())
}
