//! Path confinement for the MCP server.
//!
//! The command line is trusted: a person typed those paths. A server's paths
//! come from a model, so each one is resolved against a root and refused if it
//! escapes. Reading is allowed inside the root; writing also needs
//! `--allow-write`, and the target directory must already exist.
use std::path::{Path, PathBuf};

use crate::errors::CliError;

/// The largest data file a tool will read.
const MAX_READ_BYTES: u64 = 64 * 1024 * 1024;

/// The largest image a tool will return.
const MAX_WRITE_BYTES: u64 = 32 * 1024 * 1024;

/// A root directory, and whether writing is allowed at all.
#[derive(Debug, Clone)]
pub struct Sandbox {
    root: PathBuf,
    allow_write: bool,
}

impl Sandbox {
    /// Confine to `root`, which must exist.
    ///
    /// # Errors
    ///
    /// Fails when the root does not exist or cannot be canonicalised.
    pub fn new(root: &Path, allow_write: bool) -> Result<Self, CliError> {
        let root = root.canonicalize().map_err(|error| {
            CliError::usage(format!(
                "the root {} is not a directory: {error}",
                root.display()
            ))
        })?;
        if !root.is_dir() {
            return Err(CliError::usage(format!(
                "the root {} is not a directory",
                root.display()
            )));
        }
        Ok(Self { root, allow_write })
    }

    /// Resolve a path the caller asked for, refusing anything outside the root.
    ///
    /// The path is canonicalised *before* the check, so `../` and symlinks cannot
    /// be used to climb out.
    ///
    /// # Errors
    ///
    /// Fails when the file does not exist, is larger than the read limit, or
    /// resolves outside the root.
    pub fn read(&self, requested: &str) -> Result<PathBuf, CliError> {
        let candidate = self.root.join(requested);
        let resolved = candidate
            .canonicalize()
            .map_err(|error| CliError::data(format!("cannot read `{requested}`: {error}")))?;

        if !resolved.starts_with(&self.root) {
            return Err(CliError::data(format!(
                "`{requested}` is outside the allowed root {}, which is the only place this server reads",
                self.root.display()
            )));
        }

        let size = std::fs::metadata(&resolved)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if size > MAX_READ_BYTES {
            return Err(CliError::data(format!(
                "`{requested}` is {size} bytes; the limit is {MAX_READ_BYTES}"
            )));
        }

        Ok(resolved)
    }

    /// Resolve a path to write to.
    ///
    /// Writing is off unless the server was started with `--allow-write`, and the
    /// target's directory must already exist inside the root — the sandbox never
    /// creates directories, because `mkdir -p` on model-provided input is how
    /// surprises happen.
    ///
    /// # Errors
    ///
    /// Fails when writing is disabled, the directory is missing, or the target
    /// resolves outside the root.
    pub fn write(
        &self,
        requested: &str,
        overwrite: bool,
        bytes: usize,
    ) -> Result<PathBuf, CliError> {
        if !self.allow_write {
            return Err(CliError::usage(
                "writing is disabled; start the server with --allow-write to export files",
            ));
        }
        if bytes as u64 > MAX_WRITE_BYTES {
            return Err(CliError::data(format!(
                "the output is {bytes} bytes; the limit is {MAX_WRITE_BYTES}"
            )));
        }

        let candidate = self.root.join(requested);
        let parent = candidate
            .parent()
            .ok_or_else(|| CliError::usage(format!("`{requested}` has no directory")))?;
        let parent = parent
            .canonicalize()
            .map_err(|error| CliError::data(format!("cannot write `{requested}`: {error}")))?;
        if !parent.starts_with(&self.root) {
            return Err(CliError::data(format!(
                "`{requested}` is outside the allowed root {}",
                self.root.display()
            )));
        }

        if candidate.exists() && !overwrite {
            return Err(CliError::usage(format!(
                "`{requested}` already exists; pass `overwrite: true` to replace it"
            )));
        }

        Ok(candidate)
    }
}
