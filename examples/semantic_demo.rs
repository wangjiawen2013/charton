//! One chart, three documents.
//!
//! The drawing is a band-plus-line chart: a filled interval around a series.
//! It is exported three times with different [`SvgOptions`], which change how
//! the document is written, never what it draws:
//!
//! 1. the default document, with semantic classes and image-level accessibility;
//! 2. an inline document for a styled page: document-level accessibility, a
//!    readable id namespace, and a custom texture the page stylesheet paints
//!    the band with;
//! 3. a minimal document with no classes and no accessibility metadata.
//!
//! Run with `cargo run --example semantic_demo`, then open
//! `target/test-output/semantic_demo.html`. The page carries one stylesheet and
//! no JavaScript: CSS custom properties set the colours and the typography
//! scale, and selectors such as `.charton-tick-label` and `[data-layer="0"]`
//! restyle the chart by role and by layer. The same page is written to
//! `docs/src/export/semantic_demo.html`, where the book embeds it.

use charton::prelude::*;
use std::error::Error;

/// A fine diagonal texture for the uncertainty band.
///
/// The fragment is inserted into the SVG `<defs>` block unchanged, so the page
/// stylesheet can paint any filled element with `fill: url(#demo-band)`.
const BAND_PATTERN: &str = r##"<pattern id="demo-band" width="8" height="8" patternUnits="userSpaceOnUse"><rect width="8" height="8" fill="#2563eb" fill-opacity="0.10" /><path d="M0 8 L8 0" stroke="#2563eb" stroke-opacity="0.22" stroke-width="1" /></pattern>"##;

fn main() -> Result<(), Box<dyn Error>> {
    // Eleven years of a measurement, sampled densely enough that the band and
    // the line read as curves rather than as a staircase. `chart!` names the
    // columns after the variables, so every layer receives its own copy of the
    // data.
    let sample_count = 121;
    let year: Vec<f64> = (0..sample_count)
        .map(|i| 2013.0 + 11.0 * i as f64 / (sample_count - 1) as f64)
        .collect();
    let mid: Vec<f64> = year
        .iter()
        .map(|x| {
            let t = x - 2013.0;
            13.0 + 0.9 * t + 1.1 * (t * 0.8).sin()
        })
        .collect();
    let spread: Vec<f64> = year
        .iter()
        .map(|x| {
            let t = x - 2013.0;
            2.6 + 0.8 * (t * 0.7 + 0.5).sin()
        })
        .collect();
    let low: Vec<f64> = mid.iter().zip(&spread).map(|(m, s)| m - s).collect();
    let high: Vec<f64> = mid.iter().zip(&spread).map(|(m, s)| m + s).collect();

    // Layer 1: the uncertainty band. A closed polygon is the upper edge from
    // left to right followed by the lower edge back again; `mark_polygon`
    // connects rows in order and closes the loop, so the two edges become one
    // filled region. The whole region is a single path, which is exactly what
    // makes it addressable by a stylesheet.
    let mut band_year = year.clone();
    band_year.extend(year.iter().rev().copied());
    let mut band_value = high.clone();
    band_value.extend(low.iter().rev().copied());
    let band_group = vec!["band"; band_year.len()];

    let band = chart!(band_year, band_value, band_group)?
        .mark_polygon()?
        .configure_polygon(|mark| mark.with_fill("#dbeafe").with_stroke("none"))
        .encode((
            alt::x("band_year"),
            alt::y("band_value"),
            alt::path_group("band_group"),
        ))?;

    // Layer 2: the series itself.
    let line = chart!(year, mid)?
        .mark_line()?
        .encode((alt::x("year"), alt::y("mid")))?;

    // The band is drawn first so the line stays on top.
    let chart = band
        .and(line)
        .with_title("Annual output")
        .with_description("Annual output with its uncertainty band.")
        .with_x_label("Year")
        .with_y_label("GWh");

    // 1. The default document. Semantic classes stay on, and image-level
    //    accessibility wraps the whole drawing in one `role="img"` node with
    //    the title and description. This is the form a standalone asset or an
    //    `<img>` embed needs.
    chart
        .clone()
        .save("target/test-output/semantic_default.svg")?;

    // 2. A document meant to be inlined into a styled page. Document-level
    //    accessibility exposes the marks as objects and hides purely decorative
    //    groups; the explicit id prefix makes the generated ids readable; and
    //    the custom `<defs>` fragment gives the page a pattern it can reference
    //    from CSS.
    let styled = chart.to_svg_with(
        &SvgOptions::default()
            .with_a11y(A11y::Document)
            .with_id_prefix("demo")
            .with_defs(BAND_PATTERN),
    )?;

    let demo_page = page(&styled);
    std::fs::write("target/test-output/semantic_demo.html", &demo_page)?;
    std::fs::create_dir_all("docs/src/export")?;
    std::fs::write("docs/src/export/semantic_demo.html", &demo_page)?;

    // 3. A byte-minimal asset: no semantic classes and no accessibility
    //    metadata, for embedding where the host document already describes the
    //    image.
    let plain = chart.to_svg_with(&SvgOptions::default().without_classes().with_a11y(A11y::Off))?;
    std::fs::write("target/test-output/semantic_plain.svg", plain)?;

    Ok(())
}

/// A page whose stylesheet restyles the inlined chart by role, layer and value.
///
/// The stylesheet only reads the classes and data attributes charton writes:
/// one set of custom properties drives the light/dark switch, the typography
/// scale and the series colours. The page is meant to be embedded as is (the
/// book does so in an `<iframe>`), so it carries no heading or prose.
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
    --band: url(#demo-band);
    --highlight: #dc2626;
    --text-base: 16px;
    --text-title: 17px;
    --text-axis-title: 12px;
    --text-tick: 11px;
  }}
  html, body {{ margin: 0; padding: 0; background: var(--surface); }}
  body {{ font: var(--text-base)/1.6 "Inter", system-ui, sans-serif; color: var(--ink); }}
  svg.charton {{ display: block; width: 100%; height: auto; }}

  /* Layer 0 is the band: one filled path, painted with a pattern from <defs>. */
  [data-layer="0"] path {{ fill: var(--band); stroke: none; }}
  /* Layer 1 is the series: a stroked path with no area of its own. */
  [data-layer="1"] path {{ fill: none; stroke: var(--accent); stroke-width: 2.5; }}

  /* The pattern is part of the inline document, so the page can colour it too. */
  #demo-band rect {{ fill: var(--accent); fill-opacity: 0.10; }}
  #demo-band path {{ stroke: var(--accent); stroke-opacity: 0.22; }}

  /* Chrome and type are addressed by role, never by element type alone. */
  .charton-background rect {{ fill: var(--surface); }}
  .charton-title text {{ fill: var(--ink); font-size: var(--text-title); }}
  .charton-axis-line path, .charton-tick path {{ stroke: var(--muted); }}
  .charton-axis-title text {{ fill: var(--muted); font-size: var(--text-axis-title); }}
  .charton-tick-label text {{ fill: var(--muted); font-size: var(--text-tick); }}

  /* A single observation: the 2020 tick label, independent of the rest of the
     axis. The role rule above sets the default colour, so this one has to be
     at least as specific to override it. */
  .charton-tick-label text[data-value="2020"] {{ fill: var(--highlight); font-weight: 700; }}

  @media (prefers-color-scheme: dark) {{
    :root {{ --ink: #e5e7eb; --muted: #9ca3af; --accent: #60a5fa; --surface: #111827; --highlight: #f87171; }}
  }}
  @media print {{
    .charton-grid, .charton-frame {{ display: none; }}
    [data-layer="1"] path {{ stroke: #000; stroke-dasharray: 3 2; }}
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
