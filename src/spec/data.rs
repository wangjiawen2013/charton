//! Building a dataset from a spec's inline rows.
//!
//! No filesystem, no network. The caller resolves `data.url`, because only the
//! entry point knows what it is allowed to read; this module handles
//! `data.values` and nothing else.
//!
//! Column types are inferred per column: all numbers become continuous, all
//! booleans categorical, anything else text. One text cell makes the whole
//! column text, which never loses a value.
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::spec::types::{Data, InlineValue, Spec};

/// Build the dataset a spec carries inline.
///
/// - `Ok(None)` — the spec uses `data.url`; the caller resolves it.
/// - `Ok(Some(dataset))` — the rows in `data.values`.
/// - `Err(_)` — an internal inconsistency (column lengths). Validation rejects
///   the inputs that could cause it, so a caller may treat this as a bug.
///
/// Column types are inferred from the cells: all numbers become a continuous
/// column, booleans a categorical one, anything else text. A single text cell
/// makes the whole column text, which is the only choice that never loses a
/// value.
pub fn inline_dataset(data: &Data) -> Result<Option<Dataset>, ChartonError> {
    let Some(rows) = data.values.as_ref() else {
        return Ok(None);
    };
    if rows.is_empty() {
        return Ok(None);
    }

    let mut names = column_names(rows);
    names.sort();

    let mut dataset = Dataset::new();
    for name in names {
        let cells: Vec<Option<&InlineValue>> = rows.iter().map(|row| row.get(&name)).collect();
        dataset.add_column(name, column_of(&cells))?;
    }
    Ok(Some(dataset))
}

/// Convenience wrapper: the inline dataset of a whole spec, if it has one.
pub fn inline_dataset_of(spec: &Spec) -> Result<Option<Dataset>, ChartonError> {
    match spec.data.as_ref() {
        Some(data) => inline_dataset(data),
        None => Ok(None),
    }
}

/// Every column named by any row, in first-seen order.
fn column_names(rows: &[crate::spec::types::InlineRow]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for row in rows {
        for name in row.keys() {
            if !names.iter().any(|seen| seen == name) {
                names.push(name.clone());
            }
        }
    }
    names
}

/// Pick the narrowest column type that can hold every cell.
fn column_of(cells: &[Option<&InlineValue>]) -> ColumnVector {
    let mut has_text = false;
    let mut has_bool = false;
    let mut has_number = false;

    for value in cells.iter().flatten() {
        match value {
            InlineValue::Text(_) => has_text = true,
            InlineValue::Bool(_) => has_bool = true,
            InlineValue::Number(_) => has_number = true,
            InlineValue::Null(()) => {}
        }
    }

    if has_text || (has_bool && has_number) {
        // Mixed content: text is the only representation that keeps every cell.
        return ColumnVector::from(
            cells
                .iter()
                .map(|cell| match cell {
                    None | Some(InlineValue::Null(())) => None,
                    Some(value) => Some(text_of(value)),
                })
                .collect::<Vec<Option<String>>>(),
        );
    }

    if has_bool {
        return ColumnVector::from(
            cells
                .iter()
                .map(|cell| match cell {
                    Some(InlineValue::Bool(value)) => Some(*value),
                    _ => None,
                })
                .collect::<Vec<Option<bool>>>(),
        );
    }

    ColumnVector::from(
        cells
            .iter()
            .map(|cell| match cell {
                Some(InlineValue::Number(value)) => Some(*value),
                _ => None,
            })
            .collect::<Vec<Option<f64>>>(),
    )
}

/// A cell rendered as text, for mixed columns.
fn text_of(value: &InlineValue) -> String {
    match value {
        InlineValue::Null(()) => String::new(),
        InlineValue::Bool(value) => value.to_string(),
        InlineValue::Number(value) => value.to_string(),
        InlineValue::Text(value) => value.clone(),
    }
}
