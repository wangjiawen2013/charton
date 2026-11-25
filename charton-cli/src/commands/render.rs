//! `charton render` — a document into an image.

use charton::spec::from_json;

use crate::cli::RenderArgs;
use crate::commands;
use crate::errors::CliError;
use crate::input;
use crate::output;

/// Run `charton render`.
///
/// # Errors
///
/// Fails on unreadable input (exit 3), an invalid spec (exit 4), or a failed
/// render (exit 5).
pub fn run(args: RenderArgs) -> Result<(), CliError> {
    let text = input::read_text(&args.spec)?;
    let spec = from_json(&text).map_err(|errors| CliError::spec(errors, false))?;
    let data = input::dataset_for(&spec, &args.spec)?;

    let chart = commands::build_chart(&spec, data)?;
    output::write_chart(&chart, args.output.output.as_deref(), args.output.force)?;
    Ok(())
}
