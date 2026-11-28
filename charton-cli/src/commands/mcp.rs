//! `charton mcp` — serve the tools over stdio.

use crate::cli::McpArgs;
use crate::errors::CliError;
use crate::mcp;
use crate::sandbox::Sandbox;

/// Run `charton mcp`.
///
/// The sandbox is built before anything is read: a server that starts and then
/// discovers it cannot confine itself is a server that has already answered.
///
/// # Errors
///
/// Fails when the data directory is not a directory, or when the transport fails.
pub fn run(args: McpArgs) -> Result<(), CliError> {
    let sandbox = Sandbox::new(&args.data_dir, args.allow_write)?;
    mcp::serve(sandbox)
}
