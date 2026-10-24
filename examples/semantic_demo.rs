//! Renders the same band-plus-line chart three ways, to show how the exported
//! document differs while the picture does not.
//!
//! Run with `cargo run --example semantic_demo`, then open
//! `target/test-output/semantic_demo.html` in a browser: the page's stylesheet
//! restyles the inline SVG without re-rendering it.
//!
//! It also refreshes `docs/src/export/semantic_demo.html`, the page the book
//! embeds: the same trick, applied to the documentation itself.

use charton::prelude::*;
use std::error::Error;

const HATCH: &str = r##"<pattern id="demo-hatch" width="6" height="6" patternUnits="userSpaceOnUse"><path d="M0 6 L6 0" stroke="#9aa4b2" stroke-width="1" /></pattern>"##;

fn main() -> Result<(), Box<dyn Error>> {
    // Twelve years of a measurement with a lower and an upper bound. The
    // `chart!` macro names columns after the variables, hence the distinct
    // names per layer.
    let year: Vec<f64> = (0..12).map(|i| 2013.0 + i as f64).collect();
    let low: Vec<f64> = (0..12).map(|i| 10.0 + i as f64).collect();
    let mid: Vec<f64> = low.iter().map(|v| v + 3.0).collect();
    let std: Vec<f64> = (0..12).map(|i| 2.0 + (i % 4) as f64).collect();

    // Layer 1: the uncertainty band, as an interval rule per year.
    let band = chart!(year, low, std)?
        .transform_calculate("low", |row| Some(row.val("low")? - row.val("std")?))?
        .transform_calculate("high", |row| {
            Some(row.val("low")? + 2.0 * row.val("std")? + 3.0)
        })?
        .mark_errorbar()?
        .encode((alt::x("year"), alt::y("low"), alt::y2("high")))?;

    // Layer 2: the series itself.
    let line = chart!(year, mid)?
        .mark_line()?
        .encode((alt::x("year"), alt::y("mid")))?;

    let chart = line
        .and(band)
        .with_title("Output")
        .with_description("Monthly output with its uncertainty band.")
        .with_x_label("Year")
        .with_y_label("GWh");

    // 1. The default document: semantic classes, image-level accessibility.
    chart
        .clone()
        .save("target/test-output/semantic_default.svg")?;

    // 2. A document meant to be inlined into a styled page: navigable
    //    accessibility, an explicit id namespace, and a hatch pattern a
    //    stylesheet can reference with `fill: url(#demo-hatch)`.
    let styled = chart.to_svg_with(
        &SvgOptions::default()
            .with_a11y(A11y::Document)
            .with_id_prefix("demo")
            .with_defs(HATCH),
    )?;

    let demo_page = page(&styled);
    std::fs::write("target/test-output/semantic_demo.html", &demo_page)?;
    std::fs::create_dir_all("docs/src/export")?;
    std::fs::write("docs/src/export/semantic_demo.html", &demo_page)?;

    // 3. A byte-minimal asset for embedding in a document as an image.
    let plain = chart.to_svg_with(&SvgOptions::default().without_classes().with_a11y(A11y::Off))?;
    std::fs::write("target/test-output/semantic_plain.svg", plain)?;

    Ok(())
}

/// A page whose stylesheet restyles the inlined chart by role.
///
/// Nothing here is chart-specific: one stylesheet drives the light/dark switch,
/// the print layout and the series colors. The page is meant to be embedded as
/// is (the book does so in an `<iframe>`), so it carries no heading or prose.
fn page(svg: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Styling an inlined charton chart</title>
<style>
  :root {{
    --ink: #1f2933;
    --muted: #6b7280;
    --accent: #2563eb;
    --surface: #ffffff;
    --band: url(#demo-hatch);
  }}
  html, body {{ margin: 0; padding: 0; background: var(--surface); }}
  body {{ font: 16px/1.6 "Inter", system-ui, sans-serif; color: var(--ink); }}
  svg.charton {{ display: block; width: 100%; height: auto; }}
  /* Colours and fonts live on the drawn elements, so the rules reach them. */
  [data-layer="0"] path {{ stroke: var(--accent); stroke-width: 2.5; }}
  [data-layer="1"] path {{ fill: var(--band); }}
  .charton-tick-label text {{ font-size: 11px; fill: var(--muted); }}
  .charton-axis-title text {{ font-size: 12px; fill: var(--muted); }}
  [data-value="2020"] {{ font-weight: 700; }}
  @media (prefers-color-scheme: dark) {{
    :root {{ --ink: #e5e7eb; --muted: #9ca3af; --accent: #60a5fa; --surface: #111827; }}
  }}
  @media print {{
    .charton-grid, .charton-frame {{ display: none; }}
    .charton-mark path {{ stroke: #000; fill: none; }}
    [data-layer="1"] path {{ stroke-dasharray: 3 2; }}
  }}
</style>
</head>
<body>
{svg}
</body>
</html>
"#
    )
}
