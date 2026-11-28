//! Reading rows from a file or stdin.
//!
//! The library never reads a file; each entry point does it itself. CSV, TSV,
//! NDJSON and stdin all become the same shape — a `Data` block with inline rows
//! — so column typing follows `charton::spec::inline_dataset`.
//!
//! The format comes from the extension, or from the first non-space character
//! when reading stdin. A CSV column is numeric only when every cell in it parses
//! as a number; one word makes the whole column text.
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use charton::core::data::Dataset;
use charton::spec::{Data, InlineRow, InlineValue};

use crate::errors::CliError;

/// The rows a command loaded, and where they came from.
pub struct Loaded {
    /// The parsed table.
    pub dataset: Dataset,
    /// The rows as the spec's `data` block describes them.
    pub data: Data,
}

/// Read a table from a file or stdin.
///
/// `None` means stdin, and so does a path of `-`.
///
/// # Errors
///
/// Fails when there is no input (stdin is a terminal), when the file cannot be
/// read, or when its contents are not tabular.
pub fn load(source: Option<&Path>) -> Result<Loaded, CliError> {
    let from_stdin = source.is_none() || source.is_some_and(|path| path == Path::new("-"));

    let (text, label, format) = if from_stdin {
        if std::io::stdin().is_terminal() {
            return Err(CliError::usage(
                "no input: pass a CSV or NDJSON file, or pipe data on stdin",
            ));
        }
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| CliError::data(format!("cannot read stdin: {error}")))?;
        let format = sniff(&text);
        (text, "stdin".to_owned(), format)
    } else {
        let path = source.expect("checked above");
        let text = std::fs::read_to_string(path)
            .map_err(|error| CliError::data(format!("cannot read {}: {error}", path.display())))?;
        let format = Format::from_extension(path).unwrap_or_else(|| sniff(&text));
        (text, path.display().to_string(), format)
    };

    if text.trim().is_empty() {
        return Err(CliError::data(format!("{label} contains no rows")));
    }

    let rows = match format {
        Format::Csv { delimiter } => rows_from_csv(&text, &label, delimiter)?,
        Format::Ndjson => rows_from_ndjson(&text, &label)?,
    };

    if rows.is_empty() {
        return Err(CliError::data(format!("{label} contains no rows")));
    }

    let dataset = charton::spec::inline_dataset(&Data {
        url: None,
        values: Some(rows),
    })
    .map_err(|error| CliError::data(format!("{label}: {error}")))?
    .ok_or_else(|| CliError::data(format!("{label} contains no rows")))?;

    // A file records its own path; stdin records `-`, which both this loader and
    // `render` treat as stdin. An emitted spec therefore replays through a pipe.
    let spec_data = match source {
        Some(path) if path != Path::new("-") => Data {
            url: Some(path.to_string_lossy().into_owned()),
            values: None,
        },
        _ => Data {
            url: Some("-".to_owned()),
            values: None,
        },
    };

    Ok(Loaded {
        dataset,
        data: spec_data,
    })
}

/// Read a file as UTF-8 text, for specs and for `--data`.
///
/// # Errors
///
/// Fails when the file cannot be read.
pub fn read_text(path: &Path) -> Result<String, CliError> {
    std::fs::read_to_string(path)
        .map_err(|error| CliError::data(format!("cannot read {}: {error}", path.display())))
}

/// Resolve a document's `data.url` against the document file's directory.
///
/// The rule is fixed: relative paths are relative to the *document*, not to the
/// process's working directory, so a spec means the same thing wherever it is
/// run from.
///
/// # Errors
///
/// Fails when the spec has neither inline rows nor a path, or when the path
/// cannot be read.
pub fn dataset_for(spec: &charton::spec::Spec, spec_path: &Path) -> Result<Dataset, CliError> {
    if let Some(data) =
        charton::spec::inline_dataset_of(spec).map_err(|error| CliError::data(error.to_string()))?
    {
        return Ok(data);
    }

    let Some(url) = spec.data.as_ref().and_then(|data| data.url.as_deref()) else {
        return Err(CliError::data(
            "the spec names no data: add `url` or `values`",
        ));
    };

    // `-` names stdin, which is how a spec emitted from a pipe can be replayed
    // through another pipe.
    if url == "-" {
        return load(None).map(|loaded| loaded.dataset);
    }

    let base = spec_path.parent().unwrap_or_else(|| Path::new("."));
    let resolved: PathBuf = base.join(url);
    let candidate = if resolved.exists() {
        resolved
    } else {
        // Fall back to the working directory so a spec moved next to its data
        // still works, and name both places we looked.
        let from_cwd = PathBuf::from(url);
        if from_cwd.exists() {
            from_cwd
        } else {
            return Err(CliError::data(format!(
                "cannot find the data named by the spec: tried {} and {}",
                resolved.display(),
                from_cwd.display()
            )));
        }
    };

    load(Some(&candidate)).map(|loaded| loaded.dataset)
}

/// Tabular formats we read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    /// Comma- or tab-separated, with a header row.
    Csv { delimiter: u8 },
    /// One JSON object per line.
    Ndjson,
}

impl Format {
    fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "csv" => Some(Self::Csv { delimiter: b',' }),
            "tsv" => Some(Self::Csv { delimiter: b'\t' }),
            "ndjson" | "jsonl" => Some(Self::Ndjson),
            _ => None,
        }
    }
}

/// Guess the format of stdin: a JSON object starts with `{`.
fn sniff(text: &str) -> Format {
    if text.trim_start().starts_with('{') {
        Format::Ndjson
    } else {
        Format::Csv { delimiter: b',' }
    }
}

fn rows_from_csv(text: &str, label: &str, delimiter: u8) -> Result<Vec<InlineRow>, CliError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(false)
        .from_reader(text.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|error| CliError::data(format!("{label}: cannot read the header row: {error}")))?
        .iter()
        .map(str::to_owned)
        .collect();

    if headers.is_empty() {
        return Err(CliError::data(format!("{label}: the header row is empty")));
    }

    // One column at a time, so a column's type is decided from all its cells
    // rather than from the first one.
    let mut columns: Vec<Vec<Option<String>>> = vec![Vec::new(); headers.len()];
    for (index, record) in reader.records().enumerate() {
        let record = record
            .map_err(|error| CliError::data(format!("{label}: row {}: {error}", index + 2)))?;
        for (position, cell) in record.iter().enumerate() {
            columns[position].push(Some(cell.to_owned()).filter(|cell| !cell.is_empty()));
        }
    }

    // A column is numeric only when every cell in it parses as a number; one
    // word makes it text. Deciding per column rather than per cell is what keeps
    // an id column of "1", "2", "n/a" readable instead of half-numeric.
    let numeric: Vec<bool> = columns
        .iter()
        .map(|column| {
            let mut any = false;
            let all = column.iter().flatten().all(|cell| {
                any = true;
                cell.parse::<f64>().is_ok()
            });
            any && all
        })
        .collect();

    let rows = (0..columns.first().map_or(0, Vec::len))
        .map(|row| {
            let mut out = InlineRow::new();
            for (position, header) in headers.iter().enumerate() {
                let cell = columns[position][row].as_deref();
                let value = match (cell, numeric[position]) {
                    (None, _) => InlineValue::Null(()),
                    (Some(text), true) => InlineValue::Number(
                        text.parse().expect("checked when deciding the column type"),
                    ),
                    (Some(text), false) => InlineValue::Text(text.to_owned()),
                };
                out.insert(header.clone(), value);
            }
            out
        })
        .collect();

    Ok(rows)
}

fn rows_from_ndjson(text: &str, label: &str) -> Result<Vec<InlineRow>, CliError> {
    let mut rows = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| CliError::data(format!("{label}: line {}: {error}", index + 1)))?;
        let object = value.as_object().ok_or_else(|| {
            CliError::data(format!(
                "{label}: line {}: each line must be a JSON object",
                index + 1
            ))
        })?;

        let mut row = InlineRow::new();
        for (key, value) in object {
            row.insert(key.clone(), cell(value, label, index + 1)?);
        }
        rows.push(row);
    }

    Ok(rows)
}

/// A JSON value as a cell: scalars only.
fn cell(value: &serde_json::Value, label: &str, line: usize) -> Result<InlineValue, CliError> {
    match value {
        serde_json::Value::Null => Ok(InlineValue::Null(())),
        serde_json::Value::Bool(value) => Ok(InlineValue::Bool(*value)),
        serde_json::Value::Number(number) => number
            .as_f64()
            .map(InlineValue::Number)
            .ok_or_else(|| CliError::data(format!("{label}: line {line}: number out of range"))),
        serde_json::Value::String(text) => Ok(InlineValue::Text(text.clone())),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => Err(CliError::data(format!(
            "{label}: line {line}: nested values are not chart cells"
        ))),
    }
}
