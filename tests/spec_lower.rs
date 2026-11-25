//! End-to-end: a document becomes a chart through the same path a Rust user takes.
//!
//! The point of this file is not to check pixels. It checks three properties the
//! architecture promises:
//!
//! 1. A validated document renders.
//! 2. Rendering it twice gives the same bytes (so it can be a golden test).
//! 3. The document and the typed API produce *identical* output — there is one
//!    build path, not one per entry point.

#![cfg(feature = "spec")]

use charton::prelude::*;
use charton::spec::{codes, from_json, inline_dataset_of, lower, validate_with_data};

/// Validate, check against rows, lower and render.
fn render(json: &str) -> String {
    let spec = from_json(json).expect("the document validates");
    let data = inline_dataset_of(&spec)
        .expect("the inline dataset builds")
        .expect("the document carries inline values");
    let errors = validate_with_data(&spec, &data);
    assert!(errors.is_empty(), "data validation failed: {errors:?}");
    lower(&spec, data)
        .expect("lowering succeeds")
        .to_svg()
        .expect("rendering succeeds")
}

const LINE: &str = r##"{
    "data": { "values": [
        { "x": "Jan", "y": 12.5 },
        { "x": "Feb", "y": 15.0 },
        { "x": "Mar", "y": 9.5 }
    ]},
    "mark": "line",
    "encoding": { "x": { "field": "x" }, "y": { "field": "y" } }
}"##;

#[test]
fn a_document_renders() {
    let svg = render(LINE);
    assert!(
        svg.starts_with("<svg") || svg.contains("<svg"),
        "not an SVG"
    );
    assert!(svg.contains("<path"), "a line mark must draw a path");
    // The category labels come from the data, not from the field name.
    for label in ["Jan", "Feb", "Mar"] {
        assert!(svg.contains(label), "missing category label `{label}`");
    }
}

#[test]
fn rendering_is_deterministic() {
    // Two renders of one document are byte-identical, which is what lets the
    // SVG be a golden file instead of a fuzzy comparison.
    assert_eq!(render(LINE), render(LINE));
}

#[test]
fn a_document_and_the_typed_api_produce_the_same_output() {
    // The whole architecture rests on this: one meaning, one build path.
    // Same data, same encodings, one written as a spec and one as Rust types.
    let x = vec!["Jan", "Feb", "Mar"];
    let y = vec![12.5, 15.0, 9.5];
    let via_api = chart!(x, y)
        .expect("build")
        .mark_line()
        .expect("mark")
        .encode((alt::x("x"), alt::y("y")))
        .expect("encode")
        .to_svg()
        .expect("render");

    assert_eq!(
        via_api,
        render(LINE),
        "the spec path and the typed path diverged — there is a second renderer somewhere"
    );
}

#[test]
fn a_bar_with_an_aggregate_renders_bars() {
    let svg = render(
        r#"{
            "data": { "values": [
                { "region": "North", "month": "Jan", "amount": 4.0 },
                { "region": "South", "month": "Jan", "amount": 6.0 },
                { "region": "North", "month": "Feb", "amount": 5.0 }
            ]},
            "mark": "bar",
            "encoding": {
                "x": { "field": "month" },
                "y": { "field": "amount", "aggregate": "sum" },
                "color": { "field": "region" }
            }
        }"#,
    );
    assert!(svg.contains("<rect"), "a bar mark must draw rectangles");
    assert!(
        svg.contains("North") && svg.contains("South"),
        "legend is missing"
    );
}

#[test]
fn title_description_and_mark_colour_reach_the_output() {
    let svg = render(
        r##"{
            "data": { "values": [
                { "month": "Jan", "amount": 4.0 },
                { "month": "Feb", "amount": 6.0 }
            ]},
            "mark": { "type": "bar", "color": "#4c78a8" },
            "title": "Revenue by month",
            "description": "Two months of revenue.",
            "encoding": {
                "x": { "field": "month" },
                "y": { "field": "amount" }
            }
        }"##,
    );
    assert!(svg.contains("Revenue by month"), "title missing");
    assert!(
        svg.contains("Two months of revenue."),
        "description missing"
    );
    // The renderer normalises colours, so `#4c78a8` shows up as
    // rgba(76,120,168,1.000) — same colour, one canonical spelling in the output.
    assert!(
        svg.contains("rgba(76,120,168"),
        "the fixed mark colour was not applied"
    );
}

#[test]
fn a_mark_without_lowering_is_refused() {
    // The dialect knows `quantile_box`; this build cannot draw it. It must say so
    // here, not fail later, so that "validates" keeps meaning "renders".
    let errors = from_json(
        r#"{
            "data": { "url": "penguins.csv" },
            "charton": { "mark": "quantile_box" },
            "encoding": { "x": { "field": "body_mass" } }
        }"#,
    )
    .expect_err("must refuse");

    assert_eq!(errors[0].code, codes::UNSUPPORTED_FIELD);
    assert_eq!(errors[0].at, "/charton/mark");
}

#[test]
fn an_extension_mark_renders_through_the_same_path() {
    let svg = render(
        r#"{
            "data": { "values": [
                { "group": "a", "value": 1.0 }, { "group": "a", "value": 2.0 },
                { "group": "a", "value": 3.0 }, { "group": "b", "value": 4.0 },
                { "group": "b", "value": 5.0 }, { "group": "b", "value": 6.0 }
            ]},
            "charton": { "mark": "violin" },
            "encoding": { "x": { "field": "group" }, "y": { "field": "value" } }
        }"#,
    );
    assert!(svg.contains("<path"), "a violin is drawn as paths");
}

#[test]
fn a_filter_changes_the_rows_the_mark_sees() {
    let filtered = render(
        r#"{
            "data": { "values": [
                { "group": "keep", "value": 1.0 },
                { "group": "drop", "value": 2.0 },
                { "group": "keep", "value": 3.0 }
            ]},
            "transform": [{ "filter": { "field": "group", "equal": "keep" } }],
            "mark": "point",
            "encoding": {
                "x": { "field": "value" },
                "y": { "field": "value" },
                "color": { "field": "group" }
            }
        }"#,
    );
    assert!(
        !filtered.contains("drop"),
        "the filtered-out category must not reach the axes or the legend"
    );
    assert!(filtered.contains("keep"));
}

#[test]
fn validation_sees_the_rows_the_mark_will_see() {
    // Without a filter, the repeated x value makes `bar` ambiguous: Charton
    // would sum the two rows, Vega-Lite would draw two bars.
    let ambiguous = r#"{
        "data": { "values": [
            { "month": "Jan", "amount": 1.0 },
            { "month": "Jan", "amount": 2.0 }
        ]},
        "mark": "bar",
        "encoding": { "x": { "field": "month" }, "y": { "field": "amount" } }
    }"#;
    let spec = from_json(ambiguous).expect("the document alone is valid");
    let data = inline_dataset_of(&spec).expect("builds").expect("inline");
    let errors = validate_with_data(&spec, &data);
    assert_eq!(
        errors.first().map(|error| error.code.as_str()),
        Some(codes::DATA_SHAPE)
    );

    // The filter removes the repetition, so the same mark is now exact. If
    // validation read the original rows instead of the transformed ones, this
    // would still be flagged — that is the bug this test prevents.
    let filtered = ambiguous.replace(
        r#""mark": "bar","#,
        r#""transform": [{ "filter": { "field": "amount", "range": [0, 1.5] } }],
        "mark": "bar","#,
    );
    let spec = from_json(&filtered).expect("the document alone is valid");
    let data = inline_dataset_of(&spec).expect("builds").expect("inline");
    let errors = validate_with_data(&spec, &data);
    assert!(
        errors.is_empty(),
        "the filter made the bar exact: {errors:?}"
    );
}

#[test]
fn fold_turns_wide_columns_into_a_grouped_chart() {
    let svg = render(
        r#"{
            "data": { "values": [
                { "year": 2020, "north": 1.0, "south": 3.0 },
                { "year": 2021, "north": 2.0, "south": 4.0 }
            ]},
            "transform": [{ "fold": ["north", "south"], "as": ["region", "amount"] }],
            "mark": "line",
            "encoding": {
                "x": { "field": "year" },
                "y": { "field": "amount" },
                "color": { "field": "region" }
            }
        }"#,
    );
    // The folded column names reach the legend, which proves the created
    // columns are what the encoding resolved against.
    assert!(
        svg.contains("north") && svg.contains("south"),
        "legend missing"
    );
}

#[test]
fn transforms_compose_in_document_order() {
    // filter → aggregate → window. Each step's output is the next step's input,
    // and the encoding resolves against the *last* schema.
    let svg = render(
        r#"{
            "data": { "values": [
                { "region": "north", "sales": 1.0 },
                { "region": "south", "sales": 2.0 },
                { "region": "north", "sales": 3.0 },
                { "region": "south", "sales": 30.0 }
            ]},
            "transform": [
                { "filter": { "field": "sales", "range": [0, 10] } },
                { "aggregate": [{ "op": "sum", "field": "sales", "as": "total" }], "groupby": ["region"] },
                { "window": [{ "op": "rank", "field": "total", "as": "place" }] }
            ],
            "mark": "point",
            "encoding": {
                "x": { "field": "place" },
                "y": { "field": "total" },
                "color": { "field": "region" }
            }
        }"#,
    );
    assert!(
        svg.contains("north") && svg.contains("south"),
        "both groups survive"
    );
}

#[test]
fn a_column_dropped_by_an_earlier_transform_is_a_data_error() {
    // The window runs before the aggregate, so `place` is gone by the time the
    // encoding is resolved. Validation tracks columns through the pipeline
    // instead of assuming the input schema holds at the end.
    let spec = from_json(
        r#"{
            "data": { "values": [
                { "region": "north", "sales": 1.0 },
                { "region": "south", "sales": 2.0 }
            ]},
            "transform": [
                { "window": [{ "op": "rank", "field": "sales", "as": "place" }] },
                { "aggregate": [{ "op": "sum", "field": "sales", "as": "total" }], "groupby": ["region"] }
            ],
            "mark": "point",
            "encoding": { "x": { "field": "place" }, "y": { "field": "total" } }
        }"#,
    )
    .expect("the document alone is valid");
    let data = inline_dataset_of(&spec).expect("builds").expect("inline");

    let errors = validate_with_data(&spec, &data);
    let error = errors.first().expect("the dropped column must be reported");
    assert_eq!(error.code, codes::DATA_FIELD);
    assert_eq!(error.at, "/encoding/x/field");
}

#[test]
fn a_missing_column_says_which_ones_exist() {
    let spec = from_json(
        r#"{
            "data": { "values": [{ "month": "Jan", "amount": 1.0 }] },
            "mark": "line",
            "encoding": { "x": { "field": "month" }, "y": { "field": "revenue" } }
        }"#,
    )
    .expect("the document alone is valid");
    let data = inline_dataset_of(&spec).expect("builds").expect("inline");

    let errors = validate_with_data(&spec, &data);
    let error = errors.first().expect("the missing column must be reported");
    assert_eq!(error.code, codes::DATA_FIELD);
    assert_eq!(error.at, "/encoding/y/field");
    assert!(
        error.expected.iter().any(|column| column == "amount"),
        "the candidates must be the real columns, got {:?}",
        error.expected
    );
}
