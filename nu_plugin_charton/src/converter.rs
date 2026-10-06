//! Convert Nushell pipeline values (a table: list of records) into a charton
//! [`Dataset`].
//!
//! Each column's type is inferred from all of its rows, so a column that starts
//! with a `null` still becomes numeric when the remaining rows are numbers.

use charton::prelude::{Dataset, ctime};
use nu_protocol::{LabeledError, Span, Value};

/// A single column, already resolved to a concrete physical type.
pub enum Column {
    Int(Vec<Option<i64>>),
    Float(Vec<Option<f64>>),
    Str(Vec<Option<String>>),
    Bool(Vec<Option<bool>>),
    Datetime(Vec<Option<ctime::OffsetDateTime>>),
}

pub struct Table {
    pub columns: Vec<(String, Column)>,
    pub span: Span,
}

impl Table {
    /// Build a table from pipeline values. Every value must be a record.
    pub fn from_values(values: &[Value], span: Span) -> Result<Self, LabeledError> {
        if values.is_empty() {
            return Err(LabeledError::new("Empty pipeline input")
                .with_label("charton needs at least one row to plot", span));
        }

        // Ordered union of column names across all rows (tolerates schema drift).
        let mut names: Vec<String> = Vec::new();
        for (i, v) in values.iter().enumerate() {
            match v {
                Value::Record { val, .. } => {
                    for k in val.columns() {
                        if !names.iter().any(|n| n == k) {
                            names.push(k.clone());
                        }
                    }
                }
                other => {
                    return Err(LabeledError::new("Expected table input").with_label(
                        format!(
                            "row {i} is {}; charton expects a table (list of records)",
                            other.get_type()
                        ),
                        other.span(),
                    ));
                }
            }
        }

        if names.is_empty() {
            return Err(
                LabeledError::new("Table has no columns").with_label("nothing to plot", span)
            );
        }

        let mut columns = Vec::with_capacity(names.len());
        for name in &names {
            let cells: Vec<Option<&Value>> = values
                .iter()
                .map(|v| match v {
                    Value::Record { val, .. } => val.get(name.as_str()),
                    _ => None,
                })
                .collect();
            columns.push((name.clone(), classify(&cells)));
        }

        Ok(Table { columns, span })
    }

    /// Materialize the columns into a charton `Dataset`.
    pub fn to_dataset(&self) -> Result<Dataset, LabeledError> {
        let mut ds = Dataset::new();
        for (name, col) in &self.columns {
            let result = match col {
                Column::Int(d) => ds.add_column(name.clone(), d.clone()),
                Column::Float(d) => ds.add_column(name.clone(), d.clone()),
                Column::Str(d) => ds.add_column(name.clone(), d.clone()),
                Column::Bool(d) => ds.add_column(name.clone(), d.clone()),
                Column::Datetime(d) => ds.add_column(name.clone(), d.clone()),
            };
            result.map_err(|e| {
                LabeledError::new("Failed to build chart dataset")
                    .with_label(format!("column '{name}': {e}"), self.span)
            })?;
        }
        Ok(ds)
    }
}

fn non_null<'a>(cells: &'a [Option<&'a Value>]) -> Vec<&'a Value> {
    cells
        .iter()
        .filter_map(|c| c.as_ref().copied())
        .filter(|v| !matches!(v, Value::Nothing { .. }))
        .collect()
}

/// Decide the physical type of a column from all of its non-null values.
fn classify(cells: &[Option<&Value>]) -> Column {
    let nn = non_null(cells);

    if nn.is_empty() {
        return Column::Float(vec![None; cells.len()]);
    }

    let all_numeric = nn.iter().all(|v| {
        matches!(
            v,
            Value::Int { .. }
                | Value::Float { .. }
                | Value::Filesize { .. }
                | Value::Duration { .. }
        )
    });
    if all_numeric {
        let any_float = nn.iter().any(|v| matches!(v, Value::Float { .. }));
        return if any_float {
            Column::Float(cells.iter().map(numeric_f64).collect())
        } else {
            Column::Int(cells.iter().map(numeric_i64).collect())
        };
    }

    if nn.iter().all(|v| matches!(v, Value::Bool { .. })) {
        return Column::Bool(
            cells
                .iter()
                .map(|c| c.and_then(|v| v.as_bool().ok()))
                .collect(),
        );
    }

    if nn.iter().all(|v| matches!(v, Value::Date { .. })) {
        return Column::Datetime(
            cells
                .iter()
                .map(|c| {
                    c.and_then(|v| match v {
                        Value::Date { val, .. } => val.timestamp_nanos_opt().and_then(|ns| {
                            ctime::OffsetDateTime::from_unix_timestamp_nanos(ns as i128).ok()
                        }),
                        _ => None,
                    })
                })
                .collect(),
        );
    }

    // Everything else (including mixed types) becomes a string column so it can
    // still be used as a categorical axis.
    Column::Str(
        cells
            .iter()
            .map(|c| match c {
                None | Some(Value::Nothing { .. }) => None,
                Some(Value::String { val, .. }) => Some(val.clone()),
                Some(other) => other.coerce_string().ok(),
            })
            .collect(),
    )
}

fn numeric_i64(c: &Option<&Value>) -> Option<i64> {
    match c {
        Some(Value::Int { val, .. }) => Some(*val),
        Some(Value::Filesize { val, .. }) => Some(val.get()),
        Some(Value::Duration { val, .. }) => Some(*val),
        Some(Value::Float { val, .. }) => Some(*val as i64),
        _ => None,
    }
}

fn numeric_f64(c: &Option<&Value>) -> Option<f64> {
    match c {
        Some(Value::Int { val, .. }) => Some(*val as f64),
        Some(Value::Float { val, .. }) => Some(*val),
        Some(Value::Filesize { val, .. }) => Some(val.get() as f64),
        Some(Value::Duration { val, .. }) => Some(*val as f64),
        _ => None,
    }
}
