//! End-to-end guards for axis tick placement.
//!
//! These tests render real SVGs and inspect the tick labels, because the
//! contract is about the output: the count must follow the rendered labels,
//! must not grow with the canvas, must never stack text, and must give way to
//! explicit settings (`with_tick_count`, explicit tick values, `coord_flip`).

use charton::prelude::*;

/// One axis tick label, with the position it is anchored at.
struct LabelNode {
    position: f64,
    text: String,
}

/// All `<text>` elements of the SVG that are axis tick labels.
///
/// The theme's tick font size is 13 (titles are 15/18, legend labels 12), and
/// the axis title is the only larger text on an axis. The anchor separates the
/// two axes: the bottom one centres its labels under the tick, the left one
/// right-aligns them against it.
fn axis_labels(svg: &str, anchor: &str) -> Vec<LabelNode> {
    svg.lines()
        .filter(|line| {
            line.contains("<text") && line.contains("font-size=\"13.0\"") && line.contains(anchor)
        })
        .filter_map(|line| {
            let position = line
                .split("x=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .and_then(|value| value.parse::<f64>().ok())?;
            let body = line.split("</text>").next()?;
            let text = body.rsplit('>').next()?.to_string();
            Some(LabelNode { position, text })
        })
        .collect()
}

fn x_labels(svg: &str) -> Vec<LabelNode> {
    // Horizontal axis tick labels are centred under their tick.
    axis_labels(svg, "text-anchor=\"middle\"")
}

fn y_labels(svg: &str) -> Vec<LabelNode> {
    // Vertical axis tick labels are right-aligned against their tick.
    axis_labels(svg, "text-anchor=\"end\"")
}

/// The closest distance between two neighbouring labels.
fn closest_gap(labels: &[LabelNode]) -> f64 {
    labels
        .windows(2)
        .map(|pair| (pair[1].position - pair[0].position).abs())
        .fold(f64::INFINITY, f64::min)
}

fn numeric_points() -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..50).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();
    (x, y)
}

/// The automatic count is capped, so a huge canvas is not a wall of numbers.
const MAX_LABELS: usize = 18;

/// A larger canvas must not make the labels denser.
///
/// The tick count may still grow while the panel is small, but the *spacing*
/// between ticks has to be non-decreasing: doubling the canvas can never halve
/// the distance between labels, which is what the fixed pixel budget did.
#[test]
fn large_canvases_are_never_denser() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y) = numeric_points();

    let mut spacings = Vec::new();
    for (w, h) in [(600u32, 400u32), (1200, 800), (2400, 1600), (4800, 3200)] {
        let svg = chart!(&x, &y)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y")))?
            .with_size(w, h)
            .to_svg()?;
        let xs = x_labels(&svg).len();
        let ys = y_labels(&svg).len();

        // Whatever the canvas size, a label never needs to be closer than its
        // own text: the count stays in a human range.
        assert!(
            (3..=MAX_LABELS).contains(&xs),
            "{w}x{h}: {xs} x labels is outside the readable range"
        );
        assert!(
            (3..=MAX_LABELS).contains(&ys),
            "{w}x{h}: {ys} y labels is outside the readable range"
        );

        spacings.push((w as f64 / xs as f64, h as f64 / ys as f64));
    }

    for pair in spacings.windows(2) {
        let (small, large) = (pair[0], pair[1]);
        assert!(
            large.0 >= small.0,
            "the x axis got denser on a larger canvas: {spacings:?}"
        );
        assert!(
            large.1 >= small.1,
            "the y axis got denser on a larger canvas: {spacings:?}"
        );
    }

    Ok(())
}

/// Wide labels push their neighbours away; short ones let more through.
#[test]
fn wide_labels_reduce_the_tick_count() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..=100).map(|i| i as f64 * 10.0).collect();
    let y: Vec<f64> = x.iter().map(|v| v / 10.0).collect();

    // Same data and panel; only the label text differs.
    let plain = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_size(900, 600)
        .to_svg()?;

    let wide = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_x_label_format(LabelFormat::new().with_suffix(" kilograms"))
        .with_size(900, 600)
        .to_svg()?;

    let plain_n = x_labels(&plain).len();
    let wide_labels = x_labels(&wide);
    let wide_n = wide_labels.len();

    assert!(
        wide_n < plain_n,
        "wide labels should reduce the count ({wide_n} vs {plain_n})"
    );

    // And it must actually be collision-free: the labels read
    // "0 kilograms", so they need far more than the 20px a bare "0" needs.
    let widest_chars = wide_labels
        .iter()
        .map(|label| label.text.chars().count())
        .max()
        .unwrap_or(0);
    let needed = widest_chars as f64 * 13.0 * 0.45;
    assert!(
        closest_gap(&wide_labels) >= needed,
        "labels collide: {:?}",
        wide_labels
            .iter()
            .map(|l| (l.position, l.text.as_str()))
            .collect::<Vec<_>>()
    );

    Ok(())
}

/// An explicit count replaces the automatic placement, at any canvas size.
#[test]
fn explicit_tick_count_is_honoured() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y) = numeric_points();

    for (w, h) in [(600u32, 400u32), (2400, 1600)] {
        let svg = chart!(&x, &y)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y")))?
            .configure_theme(|theme| theme.with_tick_count(4))
            .with_size(w, h)
            .to_svg()?;

        let n = x_labels(&svg).len();
        // The scale still rounds to nice values, so a request of four can
        // produce three to six ticks.
        assert!(
            (3..=6).contains(&n),
            "{w}x{h}: explicit count of 4 produced {n} x labels"
        );
    }

    Ok(())
}

/// An explicit tick list wins over every automatic rule.
#[test]
fn explicit_tick_values_win() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y) = numeric_points();

    let svg = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_x_ticks(vec![10.0, 20.0, 30.0])
        .with_size(2400, 1600)
        .to_svg()?;

    let texts: Vec<String> = x_labels(&svg).into_iter().map(|l| l.text).collect();
    assert_eq!(texts, vec!["10", "20", "30"]);
    Ok(())
}

/// A wide log axis labels its decades' 2x/5x multiples instead of showing four
/// lonely powers in a sea of white space.
#[test]
fn a_wide_log_axis_uses_intermediate_ticks() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = vec![1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0];
    let y: Vec<f64> = (0..x.len()).map(|i| i as f64).collect();

    let narrow = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x").with_scale(Scale::Log), alt::y("y")))?
        .with_size(320, 300)
        .to_svg()?;
    let wide = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x").with_scale(Scale::Log), alt::y("y")))?
        .with_size(1200, 600)
        .to_svg()?;

    let texts = |labels: Vec<LabelNode>| -> Vec<String> {
        labels.into_iter().map(|label| label.text).collect()
    };
    assert_eq!(
        texts(x_labels(&narrow)),
        vec!["1", "10", "100", "1000"],
        "a short log axis keeps the powers"
    );
    assert_eq!(
        texts(x_labels(&wide)),
        vec!["1", "2", "5", "10", "20", "50", "100", "200", "500", "1000"],
        "a wide log axis subdivides the decades"
    );

    Ok(())
}

/// A higher output density must not add ticks.
///
/// `scale_factor` multiplies the output pixels, not the logical canvas the
/// layout is measured in. Ticks are placed in logical space, so a
/// Retina-sized asset must carry exactly the same labels at the same
/// positions — only more pixels per label.
#[test]
fn scale_factor_does_not_change_the_ticks() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y) = numeric_points();

    let render = |factor: f32| -> Result<String, Box<dyn std::error::Error>> {
        Ok(chart!(&x, &y)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y")))?
            .with_size(800, 600)
            .with_scale_factor(factor)
            .to_svg()?)
    };

    let plain = render(1.0)?;
    let retina = render(3.0)?;

    let positions = |svg: &str| -> Vec<(f64, String)> {
        x_labels(svg)
            .into_iter()
            .map(|label| (label.position, label.text))
            .collect()
    };

    assert_eq!(
        positions(&plain),
        positions(&retina),
        "the output density changed the tick placement"
    );
    Ok(())
}

/// A label almost as wide as the axis must not be drawn on top of its
/// neighbour: the automatic mode falls back to labelling only the two ends.
#[test]
fn very_long_labels_never_collide() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..=10).map(|i| i as f64 * 100_000.0).collect();
    let y: Vec<f64> = x.iter().map(|v| v / 1000.0).collect();

    let svg = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_x_label_format(LabelFormat::new().with_suffix(" milliseconds elapsed"))
        .with_size(320, 400)
        .to_svg()?;

    let labels = x_labels(&svg);
    assert!(
        labels.len() >= 2,
        "an axis should still label both ends, got {}",
        labels.len()
    );

    // The text is far wider than the 45% of an em a digit takes, so this lower
    // bound is conservative: it only catches real overlap.
    let widest = labels
        .iter()
        .map(|label| label.text.chars().count())
        .max()
        .unwrap_or(0) as f64
        * 13.0
        * 0.45;
    assert!(
        closest_gap(&labels) >= widest,
        "labels collide: {:?}",
        labels
            .iter()
            .map(|l| (l.position, l.text.as_str()))
            .collect::<Vec<_>>()
    );

    Ok(())
}

/// Explicit tick values are a contract: no automatic window and no thinning,
/// all of them are drawn.
#[test]
fn explicit_ticks_bypass_the_automatic_window() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..=10).map(|i| i as f64 * 100_000.0).collect();
    let y: Vec<f64> = x.iter().map(|v| v / 1000.0).collect();
    let many: Vec<f64> = (0..=15).map(|i| i as f64 * 60_000.0).collect();

    let svg = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_x_ticks(many.clone())
        .with_size(900, 400)
        .to_svg()?;

    assert_eq!(
        x_labels(&svg).len(),
        many.len(),
        "every explicit tick has to be drawn"
    );
    Ok(())
}

/// Explicit labels take part in the layout measurement, so a long one is not
/// pushed off the canvas by a band sized for the automatic labels.
#[test]
fn explicit_labels_widen_the_axis_band() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..10).map(|i| i as f64 * 10.0).collect();

    let svg = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_y_ticks(vec![
            (0.0, "zero points"),
            (45.0, "forty five points"),
            (90.0, "ninety points"),
        ])
        .with_size(600, 400)
        .to_svg()?;

    for label in y_labels(&svg) {
        // `anchor end` puts the anchor at the right end of the text, so the
        // whole label fits only if the anchor is at least its width from the
        // left edge. 45% of an em per character is a conservative lower bound.
        let needed = label.text.chars().count() as f64 * 13.0 * 0.45;
        assert!(
            label.position >= needed,
            "explicit label {:?} is clipped: anchor at {}, needs {}",
            label.text,
            label.position,
            needed
        );
    }
    Ok(())
}

/// `coord_flip` moves each data axis to the other physical side, and the
/// explicit tick values and axis titles move with it.
#[test]
fn a_flip_moves_ticks_and_titles_to_the_other_side() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..=10).map(|i| i as f64 * 10.0).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    let svg = chart!(&x, &y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_x_ticks(vec![10.0, 30.0, 50.0])
        .coord_flip()
        .with_size(800, 500)
        .to_svg()?;

    // The x values now label the left axis; the bottom axis is the y scale.
    let left: Vec<String> = y_labels(&svg).into_iter().map(|l| l.text).collect();
    assert_eq!(left, vec!["10", "30", "50"], "x ticks belong on the left");

    let bottom: Vec<String> = x_labels(&svg).into_iter().map(|l| l.text).collect();
    assert!(
        !bottom
            .iter()
            .any(|t| ["10", "30", "50"].contains(&t.as_str())),
        "x ticks must not stay on the bottom axis: {bottom:?}"
    );

    // Both titles are drawn, each on its new side.
    assert!(svg.contains(">x</text>"), "x title is drawn");
    assert!(svg.contains(">y</text>"), "y title is drawn");
    Ok(())
}
