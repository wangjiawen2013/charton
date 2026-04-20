//! This module provides the core bridging logic between Rust and Python.
//!
//! Structure:
//! - `bridge.rs`: Module entry point: defines the generic `Plot` container.
//! - `py_session.rs`: Python session management: handles GIL and scope injection.
//! - `converter.rs`: Data conversion: transforms `Dataset` to Python objects (Pandas/Polars).
//! - `altair.rs`: Altair implementation using PyO3.
//! - `matplotlib.rs`: Matplotlib implementation using PyO3.

pub mod altair;
pub mod converter;
pub mod matplotlib;
pub mod py_session;

use crate::core::dataset::Dataset;
use crate::error::ChartonError;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// A generic Plot container.
/// The type parameter `T` acts as a "Phantom Tag" to distinguish
/// between different renderers (Altair vs Matplotlib) at compile time.
pub struct Plot<T> {
    /// The actual data to be plotted.
    pub data: Dataset,
    /// Path to the Python interpreter (for venv/conda support).
    pub exe_path: String,
    /// The user-provided Python script.
    pub raw_plotting_code: String,
    /// Zero-sized marker to keep the generic T alive.
    _renderer: PhantomData<T>,
}

impl<T> Plot<T> {
    /// Standard constructor. Accepts the Dataset directly.
    pub fn build(data: Dataset) -> Result<Self, ChartonError> {
        Ok(Self {
            data,
            exe_path: String::new(),
            raw_plotting_code: String::new(),
            _renderer: PhantomData,
        })
    }

    /// Builder method to specify a custom Python environment path.
    pub fn with_exe_path<P: AsRef<Path>>(mut self, path: P) -> Result<Self, ChartonError> {
        let p = path.as_ref();
        // Optional: Early check for path existence
        if !p.exists() {
            return Err(ChartonError::Internal(format!(
                "Python executable not found at: {:?}",
                p
            )));
        }
        self.exe_path = p.to_string_lossy().to_string();
        Ok(self)
    }

    /// Builder method to attach the Python plotting script.
    pub fn with_plotting_code(mut self, code: &str) -> Self {
        self.raw_plotting_code = code.to_string();
        self
    }

    /// Internal utility to spin up a PythonSession using the stored exe_path.
    /// Access is restricted to the bridge crate to maintain encapsulation.
    pub(crate) fn get_session(&self) -> Result<py_session::PythonSession, ChartonError> {
        let path = if self.exe_path.is_empty() {
            None
        } else {
            Some(PathBuf::from(&self.exe_path))
        };
        py_session::PythonSession::new(path)
    }
}
