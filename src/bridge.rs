//! This module provides the core bridging logic between Rust and Python.
//!
//! Structure:
//! - `bridge.rs`: Module entry point: defines the generic `Plot` container.
//! - `py_session.rs`: Python session management: handles GIL and scope injection.
//! - `converter.rs`: Data conversion: transforms `Dataset` to Python objects.
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

/// Struct for altair renderer
pub struct Altair;

/// Struct for matplotlib renderer
pub struct Matplotlib;

/// A container that pairs a `Dataset` with its original Rust variable name.
pub struct InputData {
    pub dataset: Dataset,
    pub name: String,
}

/// Captures a `Dataset` variable, clones it, and preserves its original identifier name.
#[macro_export]
macro_rules! data {
    ($var:ident) => {
        $crate::bridge::InputData {
            // .clone() is cheap due to Arc-based columns in Dataset
            dataset: $var.clone(),
            name: stringify!($var).to_string(),
        }
    };
}

/// A generic Plot container.
pub struct Plot<T> {
    pub data: Dataset,
    pub data_name: String,
    pub exe_path: String,
    pub raw_plotting_code: String,
    _renderer: PhantomData<T>,
}

impl<T> Plot<T> {
    /// Standard constructor. Accepts InputData which pairs a Dataset with its name.
    pub fn build(input: InputData) -> Result<Self, ChartonError> {
        Ok(Self {
            data: input.dataset,
            data_name: input.name,
            exe_path: String::new(),
            raw_plotting_code: String::new(),
            _renderer: PhantomData,
        })
    }

    /// Builder method to specify a custom Python environment path.
    pub fn with_exe_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.exe_path = path.as_ref().to_string_lossy().to_string();
        self
    }

    /// Builder method to attach the Python plotting script.
    pub fn with_plotting_code(mut self, code: &str) -> Self {
        self.raw_plotting_code = code.to_string();
        self
    }

    /// Internal utility to spin up a PythonSession using the stored exe_path.
    pub(crate) fn get_session(&self) -> Result<py_session::PythonSession, ChartonError> {
        let path_opt = if self.exe_path.is_empty() {
            None
        } else {
            let p = PathBuf::from(&self.exe_path);
            if !p.exists() {
                return Err(ChartonError::Internal(format!(
                    "Python path not found: {:?}",
                    p
                )));
            }
            Some(p)
        };
        py_session::PythonSession::new(path_opt)
    }
}
