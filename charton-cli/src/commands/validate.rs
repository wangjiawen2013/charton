//! `charton validate` — check a document, and optionally its data.

use charton::spec::{ErrorReport, from_json, validate, validate_with_data};

use crate::cli::ValidateArgs;
use crate::errors::CliError;
use crate::input;

/// Run `charton validate`.
///
/// The report is the product here, so it goes to stdout in JSON mode and the
/// human summary goes to stderr. Exit code 4 means "the document has problems",
/// which is a normal outcome for a linter, not a crash.
///
/// # Errors
///
/// Fails on unreadable input (exit 3) or an invalid spec (exit 4).
pub fn run(args: ValidateArgs) -> Result<(), CliError> {
    let text = input::read_text(&args.spec)?;
    let spec = from_json(&text).map_err(|errors| CliError::spec(errors, args.json))?;

    let errors = match args.data.as_deref() {
        Some(path) => {
            let loaded = input::load(Some(path))?;
            validate_with_data(&spec, &loaded.dataset)
        }
        None => validate(&spec),
    };

    if !errors.is_empty() {
        return Err(CliError::spec(errors, args.json));
    }

    if args.json {
        let report = ErrorReport::ok();
        match serde_json::to_string_pretty(&report) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                return Err(CliError::render(format!(
                    "cannot serialise the report: {error}"
                )));
            }
        }
    } else {
        // Success is silent on stdout, so a pipeline never gets a stray line.
        eprintln!("ok: {}", args.spec.display());
    }

    Ok(())
}
