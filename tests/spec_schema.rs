//! The JSON schema is generated, never hand-written.
//!
//! A hand-maintained schema drifts from the types within one release. This test
//! makes the committed file and the Rust types the same fact in two places: if
//! they disagree, CI fails and prints the diff.
//!
//! Regenerate after an intentional type change:
//!
//! ```text
//! UPDATE_SCHEMA=1 cargo test --features spec --test spec_schema
//! ```

#![cfg(feature = "spec")]

use std::path::PathBuf;

use charton::spec::schema_json;

/// Where the published schema lives.
fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schema/charton-v0.schema.json")
}

#[test]
fn committed_schema_matches_the_types() {
    let generated = schema_json();
    let path = schema_path();

    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create schema dir");
        }
        std::fs::write(&path, &generated).expect("write schema");
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "cannot read {}: {err}\nGenerate it with: UPDATE_SCHEMA=1 cargo test --features spec --test spec_schema",
            path.display()
        )
    });

    if committed != generated {
        // Show the first divergence instead of dumping two large documents.
        let first_diff = committed
            .lines()
            .zip(generated.lines())
            .position(|(a, b)| a != b)
            .map_or_else(
                || "one file is a prefix of the other".to_owned(),
                |line| {
                    format!(
                        "first difference at line {}:\n  committed: {}\n  generated: {}",
                        line + 1,
                        committed.lines().nth(line).unwrap_or(""),
                        generated.lines().nth(line).unwrap_or("")
                    )
                },
            );
        panic!(
            "schema/charton-v0.schema.json is stale.\n{first_diff}\n\n\
             Regenerate with: UPDATE_SCHEMA=1 cargo test --features spec --test spec_schema"
        );
    }
}

#[test]
fn schema_declares_the_subset_and_its_version() {
    let schema = schema_json();
    // The URL must be Charton's own: pointing at Vega-Lite's would make editors
    // accept the full language and then fail inside Charton.
    assert!(
        !schema.contains("vega.github.io/schema/vega-lite"),
        "the published schema must not borrow Vega-Lite's URL"
    );
    assert!(
        schema.contains("charton-v0.schema.json"),
        "the schema must carry its own versioned URL"
    );
}
