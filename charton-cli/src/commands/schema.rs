//! `charton schema` — the JSON Schema, for editors and agents.

use crate::cli::SchemaArgs;
use crate::errors::CliError;
use crate::output;

/// Run `charton schema`.
///
/// # Errors
///
/// Fails when the output file exists without `--force`, or cannot be written.
pub fn run(args: SchemaArgs) -> Result<(), CliError> {
    output::write_text(
        &charton::spec::schema_json(),
        args.output.as_deref(),
        args.force,
    )
}
