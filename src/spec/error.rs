//! Spec diagnostics.
//!
//! One problem is one object:
//!
//! * `code` — stable, so tooling can branch on it;
//! * `at` — a JSON Pointer into the document;
//! * `message` — what went wrong;
//! * `hint` and `expected` — what to try instead.
//!
//! Wording may change between releases; a code may not. Every code in
//! [`codes::ALL`] is reachable from the negative corpus in `tests/spec/reject/`.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable diagnostic codes.
///
/// The three families call for different fixes: `E_UNKNOWN_*` means the word is
/// wrong and the candidates are listed; `E_UNSUPPORTED_*` means valid Vega-Lite
/// that Charton does not implement; `E_DATA_*` means the document is fine and
/// the data is not.
pub mod codes {
    /// The `$schema` is absent or names a version this build does not read.
    pub const SPEC_VERSION: &str = "E_SPEC_VERSION";
    /// A top-level or nested key is not part of the subset.
    pub const UNKNOWN_FIELD: &str = "E_UNKNOWN_FIELD";
    /// A value is not one Charton knows, and candidates can be offered.
    pub const UNKNOWN_MARK: &str = "E_UNKNOWN_MARK";
    /// A Vega-Lite channel is used that the subset does not implement.
    pub const UNKNOWN_CHANNEL: &str = "E_UNKNOWN_CHANNEL";
    /// An aggregate operator outside the supported set.
    pub const UNKNOWN_AGGREGATE: &str = "E_UNKNOWN_AGGREGATE";
    /// A scale type outside the supported set.
    pub const UNKNOWN_SCALE: &str = "E_UNKNOWN_SCALE";
    /// An operation name (in an `aggregate` or `window` transform) that does
    /// not exist.
    pub const UNKNOWN_OPERATOR: &str = "E_UNKNOWN_OPERATOR";
    /// A value has the wrong JSON type for its field.
    pub const TYPE_MISMATCH: &str = "E_TYPE_MISMATCH";
    /// A required field is missing.
    pub const MISSING_FIELD: &str = "E_MISSING_FIELD";
    /// The data source is missing, doubled, or of an unsupported kind.
    pub const DATA_SOURCE: &str = "E_DATA_SOURCE";
    /// A field referenced by an encoding does not exist in the data.
    pub const DATA_FIELD: &str = "E_DATA_FIELD";
    /// A field exists but its data type cannot serve that channel.
    pub const DATA_TYPE: &str = "E_DATA_TYPE";
    /// The rows have a shape the spec would silently misrepresent (for example
    /// repeated x values under a bar mark that aggregates them).
    pub const DATA_SHAPE: &str = "E_DATA_SHAPE";
    /// Valid Vega-Lite that Charton deliberately does not implement.
    pub const UNSUPPORTED_FIELD: &str = "E_UNSUPPORTED_FIELD";
    /// An expression string, script, or any other request to evaluate code.
    pub const UNSUPPORTED_EXPRESSION: &str = "E_UNSUPPORTED_EXPRESSION";
    /// A mark is paired with channels it cannot render.
    pub const MARK_CHANNEL: &str = "E_MARK_CHANNEL";
    /// There is no mark, or two competing ones (`mark` and `charton.mark`).
    pub const MARK_SOURCE: &str = "E_MARK_SOURCE";
    /// The document is not valid JSON at all.
    pub const JSON_SYNTAX: &str = "E_JSON_SYNTAX";
    /// A transform's output name would overwrite an existing column.
    pub const NAME_CLASH: &str = "E_NAME_CLASH";
    /// Charton's own bug: validation let something through that lowering could
    /// not build. Every occurrence must come with a new negative test.
    pub const INTERNAL: &str = "E_INTERNAL";

    /// Every code the crate can emit. Kept in sync with the constants above and
    /// asserted by the test corpus.
    pub const ALL: &[&str] = &[
        INTERNAL,
        JSON_SYNTAX,
        MARK_CHANNEL,
        MARK_SOURCE,
        NAME_CLASH,
        DATA_FIELD,
        DATA_SHAPE,
        DATA_SOURCE,
        DATA_TYPE,
        MISSING_FIELD,
        SPEC_VERSION,
        TYPE_MISMATCH,
        UNKNOWN_AGGREGATE,
        UNKNOWN_CHANNEL,
        UNKNOWN_FIELD,
        UNKNOWN_MARK,
        UNKNOWN_OPERATOR,
        UNKNOWN_SCALE,
        UNSUPPORTED_EXPRESSION,
        UNSUPPORTED_FIELD,
    ];

    /// Whether `code` is one this build can emit.
    pub fn is_known(code: &str) -> bool {
        ALL.contains(&code)
    }
}

/// One diagnostic, located inside the original spec document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SpecError {
    /// Stable code, see [`codes`].
    pub code: String,
    /// JSON Pointer to the offending node. `""` means the document root.
    pub at: String,
    /// One sentence a person can act on.
    pub message: String,
    /// How to fix it, when that is not obvious from the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Candidate values, when the failure is "this name is not one of ours".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected: Vec<String>,
}

impl SpecError {
    /// A diagnostic with a code, a location and a message.
    pub fn new(code: &str, at: impl Into<String>, message: impl Into<String>) -> Self {
        debug_assert!(
            codes::is_known(code),
            "unknown diagnostic code `{code}`: add it to codes::ALL"
        );
        Self {
            code: code.to_owned(),
            at: at.into(),
            message: message.into(),
            hint: None,
            expected: Vec::new(),
        }
    }

    /// Attach the fix suggestion.
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Attach the candidate values.
    #[must_use]
    pub fn with_expected<I, S>(mut self, expected: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.expected = expected.into_iter().map(Into::into).collect();
        self
    }

    /// Append a child key to this diagnostic's pointer.
    ///
    /// Using this instead of string formatting keeps the pointer escaping rules
    /// (`~0` for `~`, `~1` for `/`) in one place.
    #[must_use]
    pub fn at(mut self, segment: impl AsRef<str>) -> Self {
        let segment = segment.as_ref().replace('~', "~0").replace('/', "~1");
        if self.at.is_empty() {
            self.at = format!("/{segment}");
        } else {
            self.at = format!("{}/{segment}", self.at);
        }
        self
    }
}

/// The first few column names, for a hint that does not become a wall of text.
pub(crate) fn preview_columns(columns: &[String]) -> String {
    const SHOWN: usize = 8;
    if columns.len() <= SHOWN {
        return columns.join(", ");
    }
    format!(
        "{}, … ({} columns)",
        columns[..SHOWN].join(", "),
        columns.len()
    )
}

/// The JSON envelope every entry point prints with `--json`.
///
/// Success and failure share one shape so that a caller parses one thing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ErrorReport {
    /// Whether the operation succeeded.
    pub ok: bool,
    /// Empty on success; never truncated on failure.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<SpecError>,
}

impl ErrorReport {
    /// A successful report.
    pub const fn ok() -> Self {
        Self {
            ok: true,
            errors: Vec::new(),
        }
    }

    /// A failed report.
    pub const fn failed(errors: Vec<SpecError>) -> Self {
        Self {
            ok: errors.is_empty(),
            errors,
        }
    }
}
