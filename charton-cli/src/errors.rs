//! Errors, exit codes and the one place that prints them.
//!
//! Text diagnostics go to stderr. A `--json` report goes to stdout, because the
//! report is what the caller asked for.
use charton::spec::{ErrorReport, SpecError};

use crate::exit::Exit;

/// Anything a command can fail with.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// The command line was wrong.
    #[error("{0}")]
    Usage(String),

    /// The data could not be read or parsed.
    #[error("{0}")]
    Data(String),

    /// The spec is invalid. `json` remembers whether the caller asked for
    /// machine-readable output, so the report is printed exactly once, in the
    /// right stream.
    #[error("{} problem(s) in the spec", .errors.len())]
    Spec {
        /// Every diagnostic, not just the first.
        errors: Vec<SpecError>,
        /// Print the JSON envelope to stdout instead of text to stderr.
        json: bool,
    },

    /// Building, rendering or writing failed.
    #[error("{0}")]
    Render(String),
}

impl CliError {
    /// A usage error.
    pub fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    /// A data error.
    pub fn data(message: impl Into<String>) -> Self {
        Self::Data(message.into())
    }

    /// A rendering or output error.
    pub fn render(message: impl Into<String>) -> Self {
        Self::Render(message.into())
    }

    /// Spec diagnostics, in the requested output mode.
    #[must_use]
    pub fn spec(errors: Vec<SpecError>, json: bool) -> Self {
        Self::Spec { errors, json }
    }

    /// The exit code this failure maps to.
    #[must_use]
    pub const fn exit(&self) -> Exit {
        match self {
            Self::Usage(_) => Exit::Usage,
            Self::Data(_) => Exit::Data,
            Self::Spec { .. } => Exit::Spec,
            Self::Render(_) => Exit::Render,
        }
    }

    /// Print the failure.
    ///
    /// Text diagnostics go to stderr with every diagnostic spelled out; the JSON
    /// envelope goes to stdout, because `charton validate --json | jq` is the
    /// intended use.
    pub fn print(&self) {
        match self {
            Self::Spec { errors, json } => {
                if *json {
                    let report = ErrorReport::failed(errors.clone());
                    match serde_json::to_string_pretty(&report) {
                        Ok(text) => println!("{text}"),
                        Err(error) => eprintln!("charton: cannot serialise the report: {error}"),
                    }
                } else {
                    eprintln!("charton: {self}");
                    for error in errors {
                        eprintln!("  [{}] {}", error.code, at(error));
                        eprintln!("    {}", error.message);
                        if let Some(hint) = error.hint.as_ref() {
                            eprintln!("    hint: {hint}");
                        }
                        if !error.expected.is_empty() {
                            eprintln!("    expected: {}", error.expected.join(", "));
                        }
                    }
                }
            }
            other => eprintln!("charton: {other}"),
        }
    }
}

/// The pointer, or a word for the root, so the line is never blank.
fn at(error: &SpecError) -> &str {
    if error.at.is_empty() {
        "(document root)"
    } else {
        &error.at
    }
}
