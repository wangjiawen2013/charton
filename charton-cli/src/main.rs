//! The `charton` binary: argument parsing, exit codes, and nothing else.

mod cli;
mod commands;
mod errors;
mod exit;
mod input;
mod mcp;
mod output;
mod sandbox;

use std::panic::AssertUnwindSafe;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::errors::CliError;
use crate::exit::Exit;

fn main() -> ExitCode {
    // A panic is a bug, but it is still our bug: say so, point at the tracker,
    // and leave with the documented exit code instead of a bare 101.
    std::panic::set_hook(Box::new(|info| {
        eprintln!("charton: internal error (this is a bug)");
        eprintln!("  {info}");
        eprintln!("  please report it at https://github.com/wangjiawen2013/charton/issues");
    }));

    match std::panic::catch_unwind(AssertUnwindSafe(run)) {
        Ok(Ok(())) => Exit::Success.code(),
        Ok(Err(error)) => {
            error.print();
            error.exit().code()
        }
        Err(_) => Exit::Internal.code(),
    }
}

fn run() -> Result<(), CliError> {
    match Cli::parse().command {
        Command::Plot(args) => commands::plot::run(*args),
        Command::Render(args) => commands::render::run(args),
        Command::Validate(args) => commands::validate::run(args),
        Command::Schema(args) => commands::schema::run(args),
        Command::Mcp(args) => commands::mcp::run(*args),
    }
}
