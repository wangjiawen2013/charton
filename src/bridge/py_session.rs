use crate::bridge::converter::dataset_to_py;
use crate::core::dataset::Dataset;
use crate::error::ChartonError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::ffi::CString;
use std::path::PathBuf;

/// PythonSession acts as a high-level manager for the embedded Python interpreter.
/// It maintains a persistent global namespace so that data injected in one call
/// can be accessed by scripts in subsequent calls.
pub struct PythonSession {
    /// `Py<PyDict>` is a "Stable Reference" to a Python dictionary.
    /// Unlike `Bound<'py, PyDict>`, this can be stored in a struct because it
    /// doesn't carry a short-lived GIL lifetime. It lives on the Rust heap
    /// and keeps the Python object alive.
    pub globals: Py<PyDict>,
}

impl PythonSession {
    /// Creates a new session and initializes the Python interpreter.
    ///
    /// # Arguments
    /// * `custom_path` - An optional path to a specific Python executable.
    ///   Useful for virtual environments (venv) or specific installations.
    pub fn new(custom_path: Option<PathBuf>) -> Result<Self, ChartonError> {
        // --- ENV VAR SAFETY ---
        // Setting environment variables is 'unsafe' in modern Rust because it can
        // cause data races if other threads are reading env vars simultaneously.
        // We use 'unsafe' here assuming the session is initialized at startup.
        if let Some(path) = custom_path {
            unsafe {
                std::env::set_var("PYO3_PYTHON", path);
            }
        }

        // with_gil() ensures the Global Interpreter Lock is held.
        Python::with_gil(|py| {
            // We create a dedicated dictionary to act as our "Sandbox" / Global Scope.
            // This prevents our variables from polluting the real Python __main__ module.
            let globals = PyDict::new(py);

            Ok(Self {
                // .into() converts Bound<'py, PyDict> (temporary)
                // into Py<PyDict> (persistent).
                globals: globals.into(),
            })
        })
    }

    /// Takes a Rust Dataset, converts it to a Python object, and assigns it to a variable.
    ///
    /// # Example
    /// If var_name is "my_df", you can later run `print(my_df.head())` in Python.
    pub fn feed_dataset(&self, var_name: &str, dataset: &Dataset) -> Result<(), ChartonError> {
        Python::with_gil(|py| {
            // 1. Perform the columnar conversion (handled in converter.rs)
            let py_data = dataset_to_py(py, dataset)?;

            // 2. Attach the persistent globals to the current GIL lifetime.
            // .bind(py) turns the "Stable Ref" back into an "Active Handle".
            let globals = self.globals.bind(py);

            // 3. Perform the assignment: globals["var_name"] = py_data
            globals.set_item(var_name, py_data)?;

            Ok(())
        })
    }

    /// Executes arbitrary Python code within the session's global scope.
    pub fn run_code(&self, code: &str) -> Result<(), ChartonError> {
        Python::with_gil(|py| {
            let globals = self.globals.bind(py);

            // --- C-STRING COMPATIBILITY ---
            // Python's internal engine is written in C. C strings must end with a
            // null terminator (\0). Rust strings do not.
            // CString::new() validates your code and adds the \0.
            let c_code = CString::new(code).map_err(|e| {
                ChartonError::Internal(format!("Null byte found in Python code: {}", e))
            })?;

            // Execute the code.
            // The first 'globals' is for Global variables, the second for Locals.
            // Using the same dict for both makes it behave like a standard script.
            py.run(c_code.as_c_str(), Some(&globals), Some(&globals))?;

            Ok(())
        })
    }
}
