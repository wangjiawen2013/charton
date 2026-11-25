//! The negative corpus: what Charton must refuse, and where it says so.
//!
//! Each file under `tests/spec/reject/` is a document Charton must reject; the
//! expectation (code + JSON Pointer) lives in `index.json` next to them so it
//! can be reviewed without reading Rust.
//!
//! Two properties matter more than any individual case:
//!
//! * **Every code is reachable.** A diagnostic nobody can trigger is
//!   documentation that lies about the code. Codes that need data or a later
//!   milestone are listed explicitly in [`NOT_YET_REACHABLE`] and remove
//!   themselves from that list when their stage lands.
//! * **Every rejection has a location.** An error without a pointer is an error
//!   an agent cannot act on.

#![cfg(feature = "spec")]

use std::path::PathBuf;

use charton::spec::{codes, from_json, inline_dataset_of, lower, validate, validate_with_data};

fn spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/spec")
}

struct Case {
    file: String,
    code: String,
    at: String,
    /// `spec` checks the document alone; `data` also gives it rows.
    stage: String,
}

fn cases() -> Vec<Case> {
    let path = spec_dir().join("reject/index.json");
    let text = std::fs::read_to_string(&path).expect("read reject/index.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("index.json is valid JSON");

    value
        .as_array()
        .expect("index.json is an array")
        .iter()
        .map(|entry| Case {
            file: entry["file"].as_str().expect("file").to_owned(),
            code: entry["code"].as_str().expect("code").to_owned(),
            at: entry["at"].as_str().expect("at").to_owned(),
            stage: entry["stage"].as_str().unwrap_or("spec").to_owned(),
        })
        .collect()
}

#[test]
fn every_negative_case_fails_for_the_stated_reason_at_the_stated_place() {
    let mut failures = Vec::new();

    for case in cases() {
        let path = spec_dir().join("reject").join(&case.file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));

        let produced = run_case(&case, &text);
        match produced {
            Ok(()) => failures.push(format!(
                "{}: was accepted, expected {} at {}",
                case.file, case.code, case.at
            )),
            Err(errors) => {
                let Some(first) = errors.first() else {
                    failures.push(format!("{}: rejected with an empty error list", case.file));
                    continue;
                };
                if first.code != case.code {
                    failures.push(format!(
                        "{}: code was {}, expected {} ({})",
                        case.file, first.code, case.code, first.message
                    ));
                }
                if first.at != case.at {
                    failures.push(format!(
                        "{}: at was {:?}, expected {:?} ({})",
                        case.file, first.at, case.at, first.message
                    ));
                }
                if first.message.trim().is_empty() {
                    failures.push(format!("{}: empty message", case.file));
                }
            }
        }
    }

    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Run one case and return the diagnostics it produced, if it failed.
fn run_case(case: &Case, text: &str) -> Result<(), Vec<charton::spec::SpecError>> {
    let parsed = from_json(text);
    if case.stage != "data" {
        return parsed.map(|_| ());
    }

    let spec = parsed?;
    let data = inline_dataset_of(&spec)
        .expect("inline dataset builds")
        .unwrap_or_else(|| {
            panic!(
                "a data-stage case must carry inline `values`: {}",
                case.file
            )
        });
    let errors = validate_with_data(&spec, &data);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[test]
fn the_corpus_index_names_real_files_and_known_codes() {
    let reject_dir = spec_dir().join("reject");
    for case in cases() {
        assert!(
            reject_dir.join(&case.file).exists(),
            "index.json lists `{}`, which does not exist",
            case.file
        );
        assert!(
            codes::is_known(&case.code),
            "index.json uses `{}`, which is not in codes::ALL",
            case.code
        );
    }
}

/// Codes whose trigger needs a later milestone.
///
/// Removing an entry here is part of landing that milestone: the coverage test
/// below then forces a corpus case for it.
const NOT_YET_REACHABLE: &[&str] = &[
    // Needs the red line around expression strings to be implemented.
    codes::UNSUPPORTED_EXPRESSION,
    // Reserved for "validation let something through": unreachable by design.
    codes::INTERNAL,
];

#[test]
fn every_diagnostic_code_is_exercised() {
    let covered: Vec<String> = cases().into_iter().map(|case| case.code).collect();

    let missing: Vec<&str> = codes::ALL
        .iter()
        .copied()
        .filter(|code| !covered.iter().any(|used| used == code))
        .filter(|code| !NOT_YET_REACHABLE.contains(code))
        .collect();

    assert!(
        missing.is_empty(),
        "these codes exist but no corpus case can trigger them: {missing:?}\n\
         Add a file under tests/spec/reject/ and an entry to index.json."
    );
}

#[test]
fn a_near_miss_gets_a_suggestion() {
    // The plan's headline example: `linee` must produce "Did you mean `line`?".
    let text = std::fs::read_to_string(spec_dir().join("reject/unknown-mark.json")).unwrap();
    let errors = from_json(&text).expect_err("must reject");
    let error = &errors[0];

    assert_eq!(error.code, codes::UNKNOWN_MARK);
    assert_eq!(
        error.hint.as_deref(),
        Some("Did you mean `line`?"),
        "the hint must name the closest supported mark"
    );
    assert!(
        error.expected.iter().any(|mark| mark == "line"),
        "the candidates must be listed for a machine to choose from"
    );
}

#[test]
fn the_accept_corpus_is_accepted() {
    let accept_dir = spec_dir().join("accept");
    let mut failures = Vec::new();

    for entry in std::fs::read_dir(&accept_dir).expect("read accept dir") {
        let path: PathBuf = entry.expect("dir entry").path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read accept case");
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        let spec = match from_json(&text) {
            Ok(spec) => spec,
            Err(errors) => {
                failures.push(format!("{name}: {errors:?}"));
                continue;
            }
        };

        // An inline-data document is checked all the way: rows, lowering and
        // rendering. This is what keeps "validates" meaning "renders", for
        // every mark the dialect claims to support.
        if let Some(data) = inline_dataset_of(&spec).expect("inline dataset builds") {
            let errors = validate_with_data(&spec, &data);
            if !errors.is_empty() {
                failures.push(format!("{name} (with data): {errors:?}"));
                continue;
            }
            match lower(&spec, data) {
                Ok(chart) => {
                    chart
                        .to_svg()
                        .unwrap_or_else(|err| panic!("{name}: renders: {err}"));
                }
                Err(errors) => failures.push(format!("{name} (lowering): {errors:?}")),
            }
        }
    }

    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn validation_is_idempotent_and_collects_all_errors() {
    // A document with several independent problems reports all of them: one
    // round trip should be enough to fix a spec.
    let text = r#"{
        "data": { "url": "https://example.com/a.csv", "values": [] },
        "mark": "bar",
        "encoding": { "x": { "field": "a" }, "text": { "field": "b" } }
    }"#;
    let errors = from_json(text).expect_err("must reject");
    let encoded: Vec<&str> = errors.iter().map(|error| error.at.as_str()).collect();

    assert!(
        errors.len() >= 3,
        "expected several independent errors, got {encoded:?}"
    );
    assert!(
        encoded.contains(&"/data"),
        "the doubled source must be reported"
    );
    assert!(
        encoded.contains(&"/encoding"),
        "the missing y must be reported"
    );
    assert!(
        encoded.contains(&"/encoding/text"),
        "the forbidden channel must be reported"
    );

    // Checking a spec twice must not accumulate or reorder anything.
    let spec = charton::spec::from_json(
        r#"{"data":{"url":"a.csv"},"mark":"line","encoding":{"x":{"field":"a"},"y":{"field":"b"}}}"#,
    )
    .expect("valid spec");
    assert_eq!(validate(&spec), validate(&spec));
}
