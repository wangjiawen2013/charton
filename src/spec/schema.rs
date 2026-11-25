//! The published JSON Schema.
//!
//! Generated from the types, never hand-written. The `schema` command and the
//! drift test both call [`schema_json`], so there is one definition of what the
//! schema is.
use serde_json::{Value, json};

use crate::spec::types::{SPEC_SCHEMA_URL, Spec};

/// The schema for this build.
///
/// `schemars` derives the structure from the types; the identity fields are
/// stamped on afterwards so the published document carries Charton's own URL
/// and says out loud that it is a subset.
#[must_use]
pub fn schema() -> Value {
    let mut value = serde_json::to_value(schemars::schema_for!(Spec))
        .expect("a derived schema is a JSON value");

    let object = value
        .as_object_mut()
        .expect("a derived schema is a JSON object");
    object.insert("$id".to_owned(), json!(SPEC_SCHEMA_URL));
    object.insert(
        "title".to_owned(),
        json!("Charton Spec v0 (Vega-Lite v5 subset)"),
    );
    object.insert(
        "description".to_owned(),
        json!(
            "A subset of the Vega-Lite v5 specification, interpreted by Charton. \
             Fields outside this subset are errors, not silent no-ops. Charton's \
             own marks live under the `charton` namespace."
        ),
    );

    value
}

/// The schema as pretty JSON with a trailing newline — the exact bytes of the
/// file in `schema/`.
#[must_use]
pub fn schema_json() -> String {
    let mut text = serde_json::to_string_pretty(&schema()).expect("schema serialises");
    text.push('\n');
    text
}
