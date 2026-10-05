//! Locks the cross-mark lane guarantee: a dodged box plot (`mark_boxplot`, whose
//! dodge is resolved by the renderer through `Position`) and a dodged quantile
//! box (`transform_quantile_box`, whose dodge is baked by `build_lane_layout`)
//! must place their lanes at the same x positions.
//!
//! Both ultimately call the single formula in `Position::offset`, but they do so
//! in different representations (normalized category position + offset vs. an
//! index-space centre). This test pins that the two representations agree.

use charton::prelude::*;
use std::error::Error;
use std::fs;

/// x-centres of the box `<rect>`s (ignoring the background/panel rectangles).
fn rect_centres(svg: &str) -> Vec<f64> {
    let mut out = Vec::new();
    for seg in svg.split("<rect ").skip(1) {
        let end = seg
            .find("/>")
            .or_else(|| seg.find('>'))
            .unwrap_or(seg.len());
        let tag = &seg[..end];
        let attr = |key: &str| -> Option<f64> {
            let pat = format!("{key}=\"");
            let s = tag.find(&pat)? + pat.len();
            let e = tag[s..].find('"')? + s;
            tag[s..e].parse().ok()
        };
        // A box is a narrow rectangle; this skips the full-canvas background and
        // the panel/legend rectangles, which have no width in this range.
        if let (Some(x), Some(w)) = (attr("x"), attr("width"))
            && w > 0.0
            && w < 100.0
        {
            out.push(x + w / 2.0);
        }
    }
    out
}

/// x-centres of every `<path>` polygon (the quantile box and its median).
fn path_centres(svg: &str) -> Vec<f64> {
    let mut out = Vec::new();
    for seg in svg.split("<path d=\"").skip(1) {
        let end = seg.find('"').unwrap_or(seg.len());
        // Each point is written as "x y"; pull all floats and take the x's.
        let nums: Vec<f64> = seg[..end]
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|t| !t.is_empty() && *t != "-")
            .filter_map(|t| t.parse::<f64>().ok())
            .collect();
        let xs: Vec<f64> = nums.iter().step_by(2).copied().collect();
        // Keep only closed shapes (the box/median have >= 4 vertices); this
        // skips the 2-point axis and tick lines.
        if xs.len() < 4 {
            continue;
        }
        if let (Some(min), Some(max)) = (
            xs.iter().copied().reduce(f64::min),
            xs.iter().copied().reduce(f64::max),
        ) {
            out.push((min + max) / 2.0);
        }
    }
    out
}

fn rounded(mut v: Vec<f64>) -> Vec<i64> {
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    v.into_iter().map(|x| (x * 10.0).round() as i64).collect()
}

#[test]
fn dodged_boxplot_and_quantile_box_share_lanes() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("penguins")?;

    // Renderer-resolved lanes (mark_boxplot dodge).
    chart!(&ds)?
        .mark_boxplot()?
        .encode((
            alt::x("Sex"),
            alt::y("Body Mass (g)"),
            alt::color("Species"),
        ))?
        .save("./target/test-output/lane_boxplot.svg")?;

    // Transform-baked lanes (transform_quantile_box + Position::dodge). The
    // colour encoding gives both charts the same legend, so their panels have
    // identical geometry and the x positions are directly comparable.
    chart!(&ds)?
        .transform_quantile_box(
            QuantileBoxTransform::new("Body Mass (g)")
                .with_category("Sex")
                .with_group("Species")
                .with_position(Position::dodge()),
        )?
        .mark_polygon()?
        .encode((
            alt::x("x").with_category_labels("Sex"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("Species"),
        ))?
        .save("./target/test-output/lane_quantile_box.svg")?;

    let boxplot = fs::read_to_string("./target/test-output/lane_boxplot.svg")?;
    let qbox = fs::read_to_string("./target/test-output/lane_quantile_box.svg")?;

    let box_lanes = rounded(rect_centres(&boxplot));
    let qbox_lanes = rounded(path_centres(&qbox));

    assert!(
        box_lanes.len() >= 4,
        "expected several dodged boxplot lanes, got {box_lanes:?}"
    );
    assert_eq!(
        box_lanes, qbox_lanes,
        "mark_boxplot lanes and transform_quantile_box lanes must coincide"
    );

    Ok(())
}

/// x of every error-bar cap (a short horizontal `<line>`), rounded. Grid lines
/// are horizontal too but much longer, so the width window keeps only caps.
fn errorbar_lane_xs(svg: &str) -> Vec<i64> {
    let mut out = Vec::new();
    for seg in svg.split("<line ").skip(1) {
        let end = seg
            .find("/>")
            .or_else(|| seg.find('>'))
            .unwrap_or(seg.len());
        let tag = &seg[..end];
        let attr = |key: &str| -> Option<f64> {
            let pat = format!("{key}=\"");
            let s = tag.find(&pat)? + pat.len();
            let e = tag[s..].find('"')? + s;
            tag[s..e].parse().ok()
        };
        if let (Some(x1), Some(y1), Some(x2), Some(y2)) =
            (attr("x1"), attr("y1"), attr("x2"), attr("y2"))
        {
            let dx = (x1 - x2).abs();
            if (y1 - y2).abs() < 1e-6 && dx > 2.0 && dx < 20.0 {
                out.push((x1 + x2) / 2.0);
            }
        }
    }
    rounded(out)
}

/// A dodged bar and a dodged error bar are layered on the same data, so every
/// error bar must land on a bar centre. Both are renderer-path marks that call
/// the one formula in `Position::offset`; the fill/interval family shares the
/// no-gap default.
#[test]
fn dodged_bar_and_errorbar_share_lanes() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("penguins")?;

    let errorbar = chart!(&ds)?.mark_errorbar()?.encode((
        alt::x("Sex"),
        alt::y("Body Mass (g)"),
        alt::color("Species"),
    ))?;

    let bar = chart!(&ds)?.mark_bar()?.encode((
        alt::x("Sex"),
        alt::y("Body Mass (g)"),
        alt::color("Species"),
    ))?;

    errorbar
        .and(bar)
        .configure_theme(|theme| theme.with_show_legend(false))
        .save("./target/test-output/lane_bar_errorbar.svg")?;

    let svg = fs::read_to_string("./target/test-output/lane_bar_errorbar.svg")?;
    let bar_lanes = rounded(path_centres(&svg));
    let error_lanes = errorbar_lane_xs(&svg);

    assert!(!bar_lanes.is_empty(), "expected several dodged bars");
    assert!(!error_lanes.is_empty(), "expected dodged error bars");
    for x in &error_lanes {
        assert!(
            bar_lanes.contains(x),
            "error bar lane {x} must coincide with a bar lane {bar_lanes:?}"
        );
    }

    Ok(())
}
