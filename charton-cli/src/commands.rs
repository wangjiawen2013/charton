//! The four commands, and the path they share.
//!
//! `plot` does not have its own rendering path: it builds the spec a user would
//! have written, then runs the same validation and the same lowering `render`
//! does. That is what makes `--emit-spec` meaningful — the spec *is* the chart,
//! not a description of one produced by a parallel code path.

pub mod mcp;
pub mod plot;
pub mod render;
pub mod schema;
pub mod validate;

use charton::core::composite::LayeredChart;
use charton::core::data::Dataset;
use charton::spec::{Spec, lower, validate_with_data};

use crate::errors::CliError;

/// Check the spec against the rows, then build the chart.
///
/// # Errors
///
/// Returns every spec diagnostic, not just the first.
pub fn build_chart(spec: &Spec, data: Dataset) -> Result<LayeredChart, CliError> {
    let errors = validate_with_data(spec, &data);
    if !errors.is_empty() {
        return Err(CliError::spec(errors, false));
    }

    lower(spec, data).map_err(|errors| CliError::spec(errors, false))
}

/// The canonical text of a spec: pretty JSON, field order fixed, defaults
/// omitted, trailing newline. Diffable, and byte-stable enough to test.
pub fn canonical(spec: &Spec) -> Result<String, CliError> {
    let mut text = serde_json::to_string_pretty(spec)
        .map_err(|error| CliError::render(format!("cannot serialise the spec: {error}")))?;
    text.push('\n');
    Ok(text)
}
