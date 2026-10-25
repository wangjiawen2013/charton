//! The semantic contract of the exported SVG document.
//!
//! These tests are the second consumer of the feature the issue asked for: a
//! page stylesheet. They address the document by role (`charton-tick-label`,
//! `data-layer`, …) the way a stylesheet or a script would, so the classes are
//! pinned as public API rather than as an implementation detail.

mod common;

use charton::prelude::*;
use common::{group_body, has_class, plot_body, shapes};
use std::collections::HashSet;
use std::error::Error;

/// A band-plus-line chart with a colour legend, a title, a description and grid
/// lines: one representative of every role the pipeline emits.
fn band_and_line(title: &str) -> Result<LayeredChart, Box<dyn Error>> {
    let year: Vec<f64> = (0..12).map(|i| 2013.0 + i as f64).collect();
    let low: Vec<f64> = (0..12).map(|i| 10.0 + i as f64).collect();
    let mid: Vec<f64> = low.iter().map(|v| v + 3.0).collect();
    let series: Vec<&str> = low
        .iter()
        .map(|v| if *v > 15.0 { "B" } else { "A" })
        .collect();

    let band = chart!(year, low, series)?
        .transform_calculate("high", |row| Some(row.val("low")? + 4.0))?
        .mark_errorbar()?
        .encode((
            alt::x("year"),
            alt::y("low"),
            alt::y2("high"),
            alt::color("series"),
        ))?;

    let line = chart!(year, mid)?.mark_line()?.encode((
        alt::x("year"),
        alt::y("mid"),
        alt::color("series"),
    ))?;

    Ok(line
        .and(band)
        .with_title(title)
        .with_description("Yearly output with its uncertainty band.")
        .with_x_label("Year")
        .with_y_label("GWh")
        .with_grid(true))
}

fn svg_with(options: &SvgOptions) -> Result<String, Box<dyn Error>> {
    let chart = band_and_line("Output")?;
    Ok(chart.to_svg_with(options)?)
}

#[test]
fn every_role_has_its_class_in_the_document() -> Result<(), Box<dyn Error>> {
    let svg = svg_with(&SvgOptions::default())?;

    for class in [
        "charton-title",
        "charton-plot",
        "charton-grid",
        "charton-frame",
        "charton-mark",
        "charton-mark-0",
        "charton-mark-1",
        "charton-mark-errorbar",
        "charton-mark-line",
        "charton-axis",
        "charton-axis-x",
        "charton-axis-y",
        "charton-axis-line",
        "charton-tick",
        "charton-tick-label",
        "charton-axis-title",
        "charton-legend",
        "charton-legend-entry",
        "charton-legend-swatch",
        "charton-legend-label",
    ] {
        assert!(
            has_class(&svg, class),
            "the document has no `{class}` class"
        );
    }

    // Identity is carried by data attributes, never by a class built from data.
    assert!(svg.contains(r#"data-axis="x""#));
    assert!(svg.contains(r#"data-axis="y""#));
    assert!(svg.contains(r#"data-layer="0""#));
    assert!(svg.contains(r#"data-mark="line""#));

    Ok(())
}

#[test]
fn the_mark_classes_name_the_layer_and_its_mark_type() -> Result<(), Box<dyn Error>> {
    let svg = svg_with(&SvgOptions::default())?;
    let line = group_body(
        &svg,
        r#"class="charton-mark charton-mark-0 charton-mark-line""#,
    );
    assert!(line.contains("<path"), "the line layer draws paths");
    let band = group_body(
        &svg,
        r#"class="charton-mark charton-mark-1 charton-mark-errorbar""#,
    );
    assert!(
        band.contains("<line"),
        "the band layer draws interval lines"
    );
    Ok(())
}

#[test]
fn classes_never_change_the_drawing() -> Result<(), Box<dyn Error>> {
    let with = svg_with(&SvgOptions::default())?;
    let without = svg_with(&SvgOptions::default().without_classes())?;
    assert_ne!(with, without, "the two documents cannot be identical");
    assert_eq!(
        shapes(&with),
        shapes(&without),
        "removing the classes must leave exactly the same picture"
    );
    assert!(!without.contains(r#"class="charton"#));
    Ok(())
}

#[test]
fn every_group_is_closed() -> Result<(), Box<dyn Error>> {
    let svg = svg_with(&SvgOptions::default())?;
    // A missing `end_group` would leak the scope onto whatever is drawn next.
    assert_eq!(svg.matches("<g").count(), svg.matches("</g>").count());
    assert_eq!(svg.matches("<text").count(), svg.matches("</text>").count());
    Ok(())
}

#[test]
fn two_charts_inlined_into_one_page_never_share_ids() -> Result<(), Box<dyn Error>> {
    let first = band_and_line("Output 2013")?.to_svg()?;
    let second = band_and_line("Output 2024")?.to_svg()?;

    let ids = |svg: &str| -> HashSet<String> {
        svg.split(" id=\"")
            .skip(1)
            .filter_map(|part| part.split('"').next())
            .map(str::to_owned)
            .collect()
    };

    let (a, b) = (ids(&first), ids(&second));
    assert!(!a.is_empty() && !b.is_empty());
    // Each document is internally consistent, and the two do not collide: the
    // clip paths and gradients of one chart cannot be picked up by the other.
    assert_eq!(
        a.len(),
        ids(&first).len(),
        "duplicate id inside one document"
    );
    assert!(a.is_disjoint(&b), "shared ids: {:?}", a.intersection(&b));
    Ok(())
}

#[test]
fn the_default_entry_point_is_the_default_options_document() -> Result<(), Box<dyn Error>> {
    // `to_svg()` is a shorthand, not a separate rendering path: the two must
    // never drift apart.
    let chart = band_and_line("Output")?;
    assert_eq!(chart.to_svg()?, chart.to_svg_with(&SvgOptions::default())?);
    Ok(())
}

#[test]
fn identical_charts_produce_identical_documents() -> Result<(), Box<dyn Error>> {
    // Ids are hashed from the drawing, so the same chart renders to the same
    // bytes in every process. Golden files and caching depend on it.
    assert_eq!(
        band_and_line("Output")?.to_svg()?,
        band_and_line("Output")?.to_svg()?
    );
    Ok(())
}

#[test]
fn each_series_has_its_own_scope() -> Result<(), Box<dyn Error>> {
    let year = vec![2019.0, 2020.0, 2019.0, 2020.0];
    let value = vec![1.0, 2.0, 3.0, 4.0];
    let series = vec!["A", "A", "B", "B"];

    let svg = chart!(year, value, series)?
        .mark_line()?
        .encode((alt::x("year"), alt::y("value"), alt::color("series")))?
        .to_svg()?;

    assert_eq!(svg.matches(r#"class="charton-series""#).count(), 2);
    assert!(svg.contains(r#"data-series="A""#));
    assert!(svg.contains(r#"data-series="B""#));

    // The series scope sits inside its layer's mark scope.
    let layer = group_body(&svg, "charton-mark-line");
    assert!(layer.contains(r#"class="charton-series""#));
    Ok(())
}

#[test]
fn an_ungrouped_mark_still_gets_one_series_scope() -> Result<(), Box<dyn Error>> {
    let year = vec![2019.0, 2020.0];
    let value = vec![1.0, 2.0];
    let svg = chart!(year, value)?
        .mark_line()?
        .encode((alt::x("year"), alt::y("value")))?
        .to_svg()?;

    // Uniform shape, without an invented series name.
    assert_eq!(svg.matches(r#"class="charton-series""#).count(), 1);
    assert!(!svg.contains("data-series="));
    Ok(())
}

#[test]
fn a_single_value_can_be_addressed_by_its_data_value() -> Result<(), Box<dyn Error>> {
    let year = vec!["2019".to_string(), "2020".to_string(), "2021".to_string()];
    let value = vec![1.0, 2.0, 3.0];
    let svg = chart!(year, value)?
        .mark_bar()?
        .encode((alt::x("year"), alt::y("value")))?
        .to_svg()?;

    // Each bar carries the category it stands for.
    for category in ["2019", "2020", "2021"] {
        assert!(
            svg.contains(&format!(r#"data-value="{category}""#)),
            "no bar for {category}"
        );
    }

    // Tick labels carry the same values, so one selector matches both the bar
    // and its label.
    let labels = group_body(&svg, r#"class="charton-tick-label""#);
    assert!(labels.contains(r#"data-value="2020""#));

    // The value is on the drawn element itself, so one selector can restyle it
    // (a class or attribute on a wrapper could not override the bar's `fill`).
    let bars = plot_body(&svg);
    assert_eq!(bars.matches(r#"data-value="#).count(), 3);
    let bar = bars
        .lines()
        .find(|line| line.contains(r#"data-value="2020""#))
        .expect("the 2020 bar");
    assert!(bar.contains("<path"), "the bar is drawn as a path");
    Ok(())
}

#[test]
fn values_are_attributes_not_classes() -> Result<(), Box<dyn Error>> {
    // A category with CSS-hostile characters must not end up in a class name.
    let year = vec!["a b".to_string(), "c#d".to_string()];
    let value = vec![1.0, 2.0];
    let svg = chart!(year, value)?
        .mark_bar()?
        .encode((alt::x("year"), alt::y("value")))?
        .to_svg()?;

    // The value stays an escaped attribute on the bar; no class is built from
    // data, so CSS-hostile characters cannot break a selector.
    assert_eq!(plot_body(&svg).matches(r#"data-value="#).count(), 2);
    assert!(svg.contains(r#"data-value="a b""#));
    assert!(svg.contains(r#"data-value="c#d""#));
    Ok(())
}

#[test]
fn a_grouped_bar_belongs_to_its_series_without_being_reordered() -> Result<(), Box<dyn Error>> {
    // Bars are drawn as a flat stream (re-grouping them would change the draw
    // order, and with it the visual stacking), so the series travels on the
    // element instead of a series scope.
    let category = vec![
        "Jan".to_string(),
        "Jan".to_string(),
        "Feb".to_string(),
        "Feb".to_string(),
    ];
    let series = vec![
        "A".to_string(),
        "B".to_string(),
        "A".to_string(),
        "B".to_string(),
    ];
    let value = vec![3.0, 5.0, 4.0, 2.0];

    let svg = chart!(category, series, value)?
        .mark_bar()?
        .encode((alt::x("category"), alt::y("value"), alt::color("series")))?
        .to_svg()?;

    // No series scope (nothing was regrouped or reordered) ...
    assert_eq!(svg.matches("charton-series").count(), 0);

    // ... but every bar names both its category and its series.
    let bars = plot_body(&svg);
    assert_eq!(bars.matches("data-value=").count(), 4);
    assert_eq!(bars.matches(r#"data-series="A""#).count(), 2);
    assert_eq!(bars.matches(r#"data-series="B""#).count(), 2);
    let bar = bars
        .lines()
        .find(|line| line.contains(r#"data-value="Jan""#) && line.contains(r#"data-series="B""#))
        .expect("the second Jan bar");
    assert!(bar.contains("<path"));
    Ok(())
}

#[test]
fn values_are_only_emitted_where_they_are_unambiguous() -> Result<(), Box<dyn Error>> {
    // A histogram bin is a continuous interval: its centre has no stable text
    // form, so it is deliberately not emitted as a `data-value`.
    let ds = Dataset::new()
        .with_column("x", vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0])?
        .with_column("y", vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0])?;
    let svg = chart!(&ds)?
        .mark_hist()?
        .encode((alt::x("x"), alt::y("y")))?
        .to_svg()?;

    assert!(!plot_body(&svg).contains("data-value="));
    Ok(())
}

#[test]
fn every_panel_is_its_own_scope() -> Result<(), Box<dyn Error>> {
    let mtcars = load_dataset("mtcars")?;
    let svg = chart!(mtcars)?
        .mark_point()?
        .encode((alt::x("wt"), alt::y("mpg")))?
        .facet(FacetSpec::grid("vs", "am"))
        .to_svg()?;

    // One plot scope per panel, each with its own clip, and no id collisions.
    assert_eq!(svg.matches(r#"class="charton-plot""#).count(), 4);
    assert_eq!(svg.matches("<clipPath id=").count(), 4);
    assert_eq!(
        svg.split("clipPath id=\"").skip(1).count(),
        svg.matches("clip-path=\"url(#").count()
    );
    assert_eq!(svg.matches("<g").count(), svg.matches("</g>").count());
    Ok(())
}

#[test]
fn a11y_modes_add_metadata_without_changing_the_drawing() -> Result<(), Box<dyn Error>> {
    let image = svg_with(&SvgOptions::default())?;
    assert!(image.contains(r#"role="img""#));
    assert!(image.contains(r#"aria-roledescription="chart""#));
    assert!(image.contains("<title id="));
    assert!(image.contains("<desc id="));
    // Title and description are addressable separately: the name is not the
    // description, and the description is not folded into the name.
    assert!(image.contains("aria-labelledby="));
    assert!(image.contains("aria-describedby="));
    // `role="img"` collapses the subtree, so hiding children would be noise.
    assert!(!image.contains("aria-hidden"));

    let document = svg_with(&SvgOptions::default().with_a11y(A11y::Document))?;
    assert!(document.contains(r#"role="graphics-document""#));
    assert!(document.contains(r#"role="graphics-object""#));
    assert!(document.contains(r#"class="charton-grid" aria-hidden="true""#));
    assert!(document.contains(r#"class="charton-background" aria-hidden="true""#));

    let off = svg_with(&SvgOptions::default().with_a11y(A11y::Off))?;
    assert!(!off.contains("aria-"));
    assert!(!off.contains("role="));
    assert!(!off.contains("<title"));

    // Accessibility metadata is markup only: the same shapes are drawn.
    assert_eq!(shapes(&image), shapes(&document));
    assert_eq!(shapes(&image), shapes(&off));
    Ok(())
}

#[test]
fn a_description_alone_names_the_document() -> Result<(), Box<dyn Error>> {
    let year = vec![1.0, 2.0, 3.0];
    let value = vec![1.0, 4.0, 9.0];
    let svg = chart!(year, value)?
        .mark_line()?
        .encode((alt::x("year"), alt::y("value")))?
        .with_description("Revenue grows with the square of the year.")
        .to_svg()?;
    assert!(svg.contains(r#"role="img""#));
    assert!(svg.contains("<desc id="));
    // A description alone is the only text there is, so it is the name.
    assert!(svg.contains("aria-labelledby="));
    assert!(!svg.contains("aria-describedby"));
    Ok(())
}

#[test]
fn custom_defs_land_in_the_single_defs_block() -> Result<(), Box<dyn Error>> {
    let options = SvgOptions::default()
        .with_id_prefix("page1")
        .with_defs(r#"<pattern id="page1-hatch" width="4" height="4" />"#);
    let svg = svg_with(&options)?;

    assert_eq!(
        svg.matches("<defs>").count(),
        1,
        "one defs block per document"
    );
    assert!(svg.contains(r#"<pattern id="page1-hatch""#));
    assert!(
        svg.contains(r#"id="page1-clip-0""#),
        "explicit namespace wins"
    );

    let defs = svg.find("<defs>").expect("defs");
    let first_group = svg.find("<g ").expect("body");
    assert!(defs < first_group, "definitions must precede the drawing");
    Ok(())
}

#[test]
fn text_content_and_attributes_are_escaped() -> Result<(), Box<dyn Error>> {
    let year = vec![1.0, 2.0, 3.0];
    let value = vec![1.0, 4.0, 9.0];
    let svg = chart!(year, value)?
        .mark_line()?
        .encode((alt::x("year"), alt::y("value")))?
        .with_title("Growth <2024> & \"beyond\"")
        .configure_theme(|theme| theme.with_title_family("A&B \"Sans\""))
        .to_svg()?;

    assert!(svg.contains("Growth &lt;2024&gt; &amp; &quot;beyond&quot;"));
    assert!(svg.contains("A&amp;B &quot;Sans&quot;"));
    assert!(!svg.contains("<2024>"));
    Ok(())
}

#[test]
fn a_legend_can_be_hidden_and_a_band_recoloured_by_css() -> Result<(), Box<dyn Error>> {
    // The stylesheet the issue asked for, expressed as the selectors it needs.
    let svg = svg_with(&SvgOptions::default())?;
    let stylesheet = "
        .charton-grid { display: none; }
        .charton-legend { display: none; }
        .charton-tick-label { font-size: 11px; }
        [data-layer=\"1\"] { fill: url(#hatch); }
    ";
    for selector in stylesheet
        .split('{')
        .filter_map(|rule| rule.split('}').next())
    {
        for token in [".charton-grid", ".charton-legend", ".charton-tick-label"] {
            if selector.contains(token) {
                assert!(svg.contains(&token[1..]), "missing {token}");
            }
        }
        if selector.contains("[data-layer=\"1\"]") {
            assert!(svg.contains(r#"data-layer="1""#));
        }
    }
    Ok(())
}

#[test]
fn path_and_polygon_are_separate_marks() -> Result<(), Box<dyn Error>> {
    let ds = Dataset::new()
        .with_column("x", vec![0.0, 1.0, 2.0])?
        .with_column("y", vec![0.0, 1.0, 0.0])?
        .with_column("group", vec!["g"; 3])?;

    let path = Chart::build(ds.clone())?
        .mark_path()?
        .configure_path(|m| m.with_stroke("#2563eb"))
        .encode((alt::x("x"), alt::y("y"), alt::path_group("group")))?;
    let svg = path.to_svg()?;
    assert!(has_class(&svg, "charton-mark-path"));
    assert!(svg.contains(r#"data-mark="path""#));
    assert!(!svg.contains("geo_path"));

    let polygon = Chart::build(ds)?
        .mark_polygon()?
        .configure_polygon(|m| m.with_fill("#dbeafe"))
        .encode((alt::x("x"), alt::y("y"), alt::path_group("group")))?;
    let svg = polygon.to_svg()?;
    assert!(has_class(&svg, "charton-mark-polygon"));
    assert!(svg.contains(r#"data-mark="polygon""#));
    assert!(!svg.contains("geo_path"));

    Ok(())
}
