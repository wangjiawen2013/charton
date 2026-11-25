//! Process exit codes.
//!
//! Fixed, because scripts branch on them. `clap` already exits with 2 on a usage
//! error, so [`Exit::Usage`] matches the convention users expect.

use std::process::ExitCode;

/// The documented exit codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// The command did what it was asked.
    Success = 0,
    /// A bug in Charton: a panic, or a spec that validated but could not build.
    Internal = 1,
    /// The command line itself was wrong (unknown flag, bad extension, no input).
    Usage = 2,
    /// The data could not be read, parsed, or resolved.
    Data = 3,
    /// The spec is invalid, or it disagrees with the data.
    Spec = 4,
    /// Building, rendering, or writing the output failed.
    Render = 5,
}

impl Exit {
    /// The code as the process exit status.
    #[must_use]
    pub fn code(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}
