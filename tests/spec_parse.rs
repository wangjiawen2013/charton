//! What the spec accepts and rejects at the boundary.
//!
//! These are shape tests: they pin the JSON dialect, not the chart. Validation
//! semantics (which mark works with which channel, whether a field exists in
//! the data) live in the negative corpus under `tests/spec/reject/`.

#![cfg(feature = "spec")]

use charton::spec::{ExtensionMark, MarkType, Spec};

fn parse(json: &str) -> Result<Spec, serde_json::Error> {
    serde_json::from_str(json)
}

const MINIMAL: &str = r#"{
    "data": { "url": "sales.csv" },
    "mark": "line",
    "encoding": {
        "x": { "field": "month" },
        "y": { "field": "revenue" }
    }
}"#;

#[test]
fn parses_a_minimal_spec() {
    let spec = parse(MINIMAL).expect("minimal spec parses");
    assert!(matches!(
        spec.mark.as_ref(),
        Some(charton::spec::Mark::Type(MarkType::Line))
    ));
    assert_eq!(spec.encoding.x.as_ref().unwrap().field, "month");
    assert_eq!(spec.encoding.y.as_ref().unwrap().field, "revenue");
    // No `$schema` means "the version this build reads".
    assert_eq!(spec.declared_version(), None);
}

#[test]
fn an_object_mark_carries_properties() {
    let spec = parse(
        r##"{
            "data": { "url": "a.csv" },
            "mark": { "type": "bar", "color": "#4c78a8", "opacity": 0.8 },
            "encoding": { "x": { "field": "a" } }
        }"##,
    )
    .expect("object mark parses");
    let charton::spec::Mark::Def(def) = spec.mark.expect("mark is present") else {
        panic!("expected the object form");
    };
    assert_eq!(def.mark_type, MarkType::Bar);
    assert_eq!(def.color.as_deref(), Some("#4c78a8"));
    assert_eq!(def.opacity, Some(0.8));
}

#[test]
fn a_misspelled_key_is_a_hard_error() {
    // Not a silent no-op: this is what makes a spec reviewable.
    let err = parse(r#"{ "data": {"url": "a.csv"}, "mark": "line", "encodng": {} }"#)
        .expect_err("unknown top-level key must fail");
    assert!(
        err.to_string().contains("encodng"),
        "the error must name the offending key, got: {err}"
    );
}

#[test]
fn charton_only_marks_live_in_their_own_namespace() {
    let spec = parse(
        r#"{
            "data": { "url": "a.csv" },
            "mark": "area",
            "charton": { "mark": "violin" },
            "encoding": { "x": { "field": "group" }, "y": { "field": "value" } }
        }"#,
    )
    .expect("extension namespace parses");
    assert_eq!(spec.charton.unwrap().mark, Some(ExtensionMark::Violin));
}

#[test]
fn a_charton_mark_may_not_borrow_the_vega_lite_name() {
    // "violin" is not a Vega-Lite mark. Putting it in `mark` must fail rather
    // than be silently accepted, so a document is never ambiguous.
    let err = parse(r#"{ "data": {"url": "a.csv"}, "mark": "violin", "encoding": {} }"#)
        .expect_err("charton marks are not valid in `mark`");
    let message = err.to_string();
    assert!(
        message.contains("violin"),
        "the error must quote the rejected name, got: {message}"
    );
}

#[test]
fn a_round_trip_keeps_the_document() {
    let spec = parse(MINIMAL).expect("parses");
    let text = serde_json::to_string(&spec).expect("serialises");
    let again = parse(&text).expect("re-parses");
    assert_eq!(spec, again);
}
