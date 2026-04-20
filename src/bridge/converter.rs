//! Data bridge conversion: core logic for converting `Dataset` to Python objects.
//!
//! This module handles the transformation of Rust-native `ColumnVector` into
//! Python-compatible formats, specifically optimized for `polars.DataFrame`.

use crate::core::dataset::{ColumnVector, Dataset};
use crate::error::ChartonError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyNone};

/// Converts a Rust `Dataset` into a Python-side data object.
///
/// It attempts to create a `polars.DataFrame` if the library is installed in the
/// Python environment. Otherwise, it falls back to a standard Python `dict` of `lists`.
///
/// # Arguments
/// * `py` - The Python token representing the held GIL (Global Interpreter Lock).
/// * `dataset` - The Rust Dataset containing columnar data.
pub fn dataset_to_py<'py>(
    py: Python<'py>,
    dataset: &Dataset,
) -> Result<Bound<'py, PyAny>, ChartonError> {
    // We start by creating a Python dictionary: {}
    // Bound<'py, PyDict> means: "A reference to a dictionary owned by Python,
    // valid as long as we hold the GIL ('py)."
    let data_dict = PyDict::new(py);

    // Iterate through the schema to preserve column names and order
    for (name, &index) in &dataset.schema {
        let col = &dataset.columns[index];

        // Convert each Rust ColumnVector into a Python List [val1, val2, None, ...]
        let py_values = column_to_py(py, col)?;

        // Insert into dictionary: data_dict["column_name"] = [values]
        data_dict.set_item(name, py_values)?;
    }

    // Optimization: Check if the user has 'polars' installed in their Python env.
    // If yes, we wrap the dictionary into a Polars DataFrame for massive speed gains.
    if let Ok(pl) = py.import("polars") {
        // Equivalent to Python: pl.from_dict(data_dict)
        let df = pl.call_method1("from_dict", (data_dict,))?;
        Ok(df)
    } else {
        // Fallback: Just return the raw dictionary.
        // .into_any() erases the PyDict type so we can return a generic PyAny.
        Ok(data_dict.into_any())
    }
}

/// Dispatches ColumnVector variants to specific conversion logic.
/// This is the bridge between Rust's Enums and Python's Dynamic Types.
fn column_to_py<'py>(
    py: Python<'py>,
    col: &ColumnVector,
) -> Result<Bound<'py, PyAny>, ChartonError> {
    match col {
        // Float64/32 handling:
        // PyO3 automatically maps f64::NAN to Python's float('nan').
        // Most plotting libs (Altair/Matplotlib) treat NaN as a "Null" value.
        ColumnVector::F64 { data } => {
            let list = PyList::new(py, data)?;
            Ok(list.into_any())
        }
        ColumnVector::F32 { data } => {
            let list = PyList::new(py, data)?;
            Ok(list.into_any())
        }

        // Integer and String handling:
        // Integers in Rust don't have a "NaN" state, so we must manually
        // check the 'validity' bitmask and insert Python 'None' for nulls.
        ColumnVector::I64 { data, validity } => {
            Ok(convert_with_mask(py, data, validity)?.into_any())
        }
        ColumnVector::I32 { data, validity } => {
            Ok(convert_with_mask(py, data, validity)?.into_any())
        }
        ColumnVector::U32 { data, validity } => {
            Ok(convert_with_mask(py, data, validity)?.into_any())
        }
        ColumnVector::String { data, validity } => {
            Ok(convert_with_mask(py, data, validity)?.into_any())
        }

        // DateTime handling:
        // Converts internal time objects into Unix Milliseconds.
        // This is the most compatible format for Vega-Lite/Altair.
        ColumnVector::DateTime { data, validity } => {
            let list = PyList::empty(py);
            for (i, val) in data.iter().enumerate() {
                if ColumnVector::is_valid_in_mask(validity, i) {
                    // Convert to milliseconds as a float or int for Python
                    let ms = val.unix_timestamp_nanos() / 1_000_000;
                    list.append(ms)?;
                } else {
                    // Insert Python's 'None'
                    list.append(PyNone::get(py))?;
                }
            }
            Ok(list.into_any())
        }
    }
}

/// A generic helper to handle the Bitmask (Validity Map) logic.
///
/// In Rust: data = [10, 0, 30], mask = [1, 0, 1] (0 is null)
/// In Python: Result = [10, None, 30]
fn convert_with_mask<'py, T>(
    py: Python<'py>,
    data: &[T],
    mask: &Option<Vec<u8>>,
) -> PyResult<Bound<'py, PyList>>
where
    T: ToPyObject, // T must be a type that PyO3 knows how to convert to Python
{
    // Pre-allocate a Python list.
    // Note: for very large datasets, using numpy would be faster than PyList.
    let list = PyList::empty(py);

    // Performance Shortcut: If there is no mask, every row is valid.
    // We can let PyO3 do a batch conversion.
    if mask.is_none() {
        for val in data {
            list.append(val)?;
        }
        return Ok(list);
    }

    // Slow Path: Check each bit to decide between Value or None.
    for (i, val) in data.iter().enumerate() {
        if ColumnVector::is_valid_in_mask(mask, i) {
            // Append the actual value
            list.append(val)?;
        } else {
            // Append Python 'None'
            list.append(PyNone::get(py))?;
        }
    }
    Ok(list)
}
