//! The seven tools.
//!
//! Each one is a thin shell over a function the CLI already uses — the
//! validator, `lower`, the data loader — so a chart rendered here is the same
//! chart. Tool failures come back in band (`isError: true` with a readable
//! message), because the caller has to decide what to try next; JSON-RPC errors
//! are for protocol mistakes only.
use base64::Engine as _;
use charton::core::data::Dataset;
use charton::spec::{
    ErrorReport, ExtensionMark, MarkType, Spec, SpecMark, ThemeName, from_json, inline_dataset_of,
    lower, mark_requirements, schema_json, validate, validate_with_data,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::input;
use crate::sandbox::Sandbox;

/// One call, with the sandbox it is confined to.
pub struct Call<'a> {
    /// The tool arguments, as sent.
    pub arguments: &'a Value,
    /// Where files may be read, and whether they may be written.
    pub sandbox: &'a Sandbox,
}

/// What a tool produced.
pub struct Outcome {
    /// Content the client will show the model.
    pub content: Vec<Content>,
    /// Whether the tool failed. The content still explains what happened.
    pub is_error: bool,
}

impl Outcome {
    /// A JSON result.
    fn json(value: &Value) -> Self {
        Self {
            content: vec![Content::Text(
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
            )],
            is_error: false,
        }
    }

    /// A plain-text result.
    fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![Content::Text(text.into())],
            is_error: false,
        }
    }

    /// A failure the caller should read.
    fn failed(message: impl Into<String>) -> Self {
        Self {
            content: vec![Content::Text(message.into())],
            is_error: true,
        }
    }
}

/// One piece of content.
pub enum Content {
    /// Text, including JSON and SVG.
    Text(String),
    /// A base64 image.
    Image {
        /// The payload.
        data: String,
        /// The media type.
        mime: String,
    },
}

/// A tool's definition.
pub struct Tool {
    /// Its name on the wire.
    pub name: &'static str,
    /// What it does, in one paragraph a model can act on.
    pub description: &'static str,
    /// The JSON Schema of its arguments.
    pub schema: fn() -> Value,
    /// The implementation.
    pub call: fn(Call<'_>) -> Outcome,
}

/// Every tool, in the order `tools/list` reports them.
pub fn all() -> &'static [Tool] {
    TOOLS
}

/// Look a tool up by name.
pub fn find(name: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|tool| tool.name == name)
}

static TOOLS: &[Tool] = &[
    Tool {
        name: "get_schema",
        description: "The JSON Schema of the chart dialect. Read this before writing a spec: \
                      it is the authoritative list of fields, marks and channels for this build.",
        schema: empty_schema,
        call: get_schema,
    },
    Tool {
        name: "profile_data",
        description: "Summarise a table: its columns, their types, missing counts, ranges or \
                      distinct values, and a sample of rows. Use it to learn the column names \
                      before writing a spec.",
        schema: profile_schema,
        call: profile,
    },
    Tool {
        name: "list_marks",
        description: "Every mark this build can draw, with the encoding channels each one \
                      requires and refuses.",
        schema: empty_schema,
        call: list_marks,
    },
    Tool {
        name: "list_themes",
        description: "The built-in themes a spec may name in its `theme` field.",
        schema: empty_schema,
        call: list_themes,
    },
    Tool {
        name: "validate_spec",
        description: "Check a chart spec. With `data`, the check also covers whether the fields \
                      exist and whether their values can serve the channel. Returns every problem \
                      with a stable code, a JSON Pointer and a hint.",
        schema: validate_schema,
        call: validate_spec,
    },
    Tool {
        name: "render_spec",
        description: "Render a chart spec to SVG (returned as text) or PNG (returned as an \
                      image). The spec is validated first; problems are returned instead of a chart.",
        schema: render_schema,
        call: render_spec,
    },
    Tool {
        name: "export_spec",
        description: "Write a spec to a file inside the allowed root. Disabled unless the server \
                      was started with --allow-write.",
        schema: export_schema,
        call: export_spec,
    },
];

// ---------------------------------------------------------------------------
// Arguments
// ---------------------------------------------------------------------------

fn empty_schema() -> Value {
    json!({"type": "object", "properties": {}, "additionalProperties": false})
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProfileArgs {
    /// Path to a CSV, TSV or NDJSON file inside the allowed root.
    path: String,
    /// How many sample rows to include. Defaults to 5.
    #[serde(default)]
    sample_rows: Option<usize>,
}

fn profile_schema() -> Value {
    to_schema(schemars::schema_for!(ProfileArgs))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ValidateArgs {
    /// The spec: either the document itself, or a path to one inside the root.
    spec: Value,
    /// Optional path to the rows, to check field names and types too.
    #[serde(default)]
    data: Option<String>,
}

fn validate_schema() -> Value {
    to_schema(schemars::schema_for!(ValidateArgs))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RenderArgs {
    /// The spec: either the document itself, or a path to one inside the root.
    spec: Value,
    /// `svg` (default) or `png`.
    #[serde(default)]
    format: Option<String>,
}

fn render_schema() -> Value {
    to_schema(schemars::schema_for!(RenderArgs))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ExportArgs {
    /// The spec: either the document itself, or a path to one inside the root.
    spec: Value,
    /// Where to write it, inside the allowed root.
    path: String,
    /// Replace an existing file.
    #[serde(default)]
    overwrite: bool,
}

fn export_schema() -> Value {
    to_schema(schemars::schema_for!(ExportArgs))
}

fn to_schema(schema: schemars::Schema) -> Value {
    serde_json::to_value(schema).unwrap_or(Value::Null)
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

fn get_schema(_call: Call<'_>) -> Outcome {
    Outcome::text(schema_json())
}

fn profile(call: Call<'_>) -> Outcome {
    let args = match parse::<ProfileArgs>(call.arguments) {
        Ok(args) => args,
        Err(outcome) => return outcome,
    };

    let path = match call.sandbox.read(&args.path) {
        Ok(path) => path,
        Err(error) => return Outcome::failed(error.to_string()),
    };
    let loaded = match input::load(Some(&path)) {
        Ok(loaded) => loaded,
        Err(error) => return Outcome::failed(error.to_string()),
    };

    Outcome::json(&profile_of(&loaded, args.sample_rows.unwrap_or(5).min(50)))
}

fn list_marks(_call: Call<'_>) -> Outcome {
    let mut marks = Vec::new();

    for mark in MarkType::ALL {
        marks.push(mark_entry(SpecMark::Vega(*mark), true));
    }
    for mark in ExtensionMark::ALL {
        // A name the dialect knows but this build cannot lower is still listed:
        // "recognised, not supported" is the answer the caller needs.
        marks.push(mark_entry(
            SpecMark::Extension(*mark),
            charton::spec::SUPPORTED_EXTENSION_MARKS.contains(mark),
        ));
    }

    Outcome::json(&json!({ "marks": marks }))
}

fn list_themes(_call: Call<'_>) -> Outcome {
    let themes: Vec<Value> = ThemeName::ALL
        .iter()
        .map(|theme| json!({ "name": theme.as_str() }))
        .collect();
    Outcome::json(&json!({ "field": "theme", "themes": themes }))
}

fn validate_spec(call: Call<'_>) -> Outcome {
    let args = match parse::<ValidateArgs>(call.arguments) {
        Ok(args) => args,
        Err(outcome) => return outcome,
    };

    let spec = match spec_from(&args.spec, call.sandbox) {
        Ok(spec) => spec,
        Err(outcome) => return outcome,
    };

    let errors = match args.data.as_deref() {
        Some(path) => match dataset_from(path, call.sandbox) {
            Ok(data) => validate_with_data(&spec, &data),
            Err(outcome) => return outcome,
        },
        None => validate(&spec),
    };

    report(errors, false)
}

fn render_spec(call: Call<'_>) -> Outcome {
    let args = match parse::<RenderArgs>(call.arguments) {
        Ok(args) => args,
        Err(outcome) => return outcome,
    };

    let png = match args.format.as_deref() {
        None | Some("svg") => false,
        Some("png") => true,
        Some(other) => {
            return Outcome::failed(format!("unknown format `{other}`; use `svg` or `png`"));
        }
    };

    let spec = match spec_from(&args.spec, call.sandbox) {
        Ok(spec) => spec,
        Err(outcome) => return outcome,
    };
    let data = match dataset_for(&spec, call.sandbox) {
        Ok(data) => data,
        Err(outcome) => return outcome,
    };

    let errors = validate_with_data(&spec, &data);
    if !errors.is_empty() {
        return report(errors, true);
    }

    let chart = match lower(&spec, data) {
        Ok(chart) => chart,
        Err(errors) => return report(errors, true),
    };

    if png {
        return match chart.to_png() {
            Ok(bytes) => Outcome {
                content: vec![Content::Image {
                    data: base64::engine::general_purpose::STANDARD.encode(bytes),
                    mime: "image/png".to_owned(),
                }],
                is_error: false,
            },
            Err(error) => Outcome::failed(format!("cannot render PNG: {error}")),
        };
    }

    match chart.to_svg() {
        Ok(svg) => Outcome::text(svg),
        Err(error) => Outcome::failed(format!("cannot render SVG: {error}")),
    }
}

fn export_spec(call: Call<'_>) -> Outcome {
    let args = match parse::<ExportArgs>(call.arguments) {
        Ok(args) => args,
        Err(outcome) => return outcome,
    };

    let spec = match spec_from(&args.spec, call.sandbox) {
        Ok(spec) => spec,
        Err(outcome) => return outcome,
    };

    // A spec that cannot render is not worth saving: export the validated form.
    let errors = validate(&spec);
    if !errors.is_empty() {
        return report(errors, true);
    }

    let text = match serde_json::to_string_pretty(&spec) {
        Ok(mut text) => {
            text.push('\n');
            text
        }
        Err(error) => return Outcome::failed(format!("cannot serialise the spec: {error}")),
    };

    let target = match call.sandbox.write(&args.path, args.overwrite, text.len()) {
        Ok(target) => target,
        Err(error) => return Outcome::failed(error.to_string()),
    };

    match std::fs::write(&target, text) {
        Ok(()) => Outcome::json(&json!({ "written": target.display().to_string() })),
        Err(error) => Outcome::failed(format!("cannot write {}: {error}", target.display())),
    }
}

// ---------------------------------------------------------------------------
// Shared plumbing
// ---------------------------------------------------------------------------

/// Deserialize arguments, turning a bad shape into a readable failure.
fn parse<T>(arguments: &Value) -> Result<T, Outcome>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(arguments.clone())
        .map_err(|error| Outcome::failed(format!("bad arguments: {error}")))
}

/// A spec, given either inline or as a path inside the root.
fn spec_from(value: &Value, sandbox: &Sandbox) -> Result<Spec, Outcome> {
    match value {
        Value::String(path) => {
            let path = sandbox
                .read(path)
                .map_err(|error| Outcome::failed(error.to_string()))?;
            let text = std::fs::read_to_string(&path).map_err(|error| {
                Outcome::failed(format!("cannot read {}: {error}", path.display()))
            })?;
            from_json(&text).map_err(|errors| report(errors, true))
        }
        Value::Object(_) => {
            let text = serde_json::to_string(value)
                .map_err(|error| Outcome::failed(format!("cannot read the spec: {error}")))?;
            from_json(&text).map_err(|errors| report(errors, true))
        }
        _ => Err(Outcome::failed("`spec` must be an object or a path")),
    }
}

/// The rows a spec names, read inside the sandbox.
fn dataset_for(spec: &Spec, sandbox: &Sandbox) -> Result<Dataset, Outcome> {
    if let Some(data) =
        inline_dataset_of(spec).map_err(|error| Outcome::failed(error.to_string()))?
    {
        return Ok(data);
    }

    let url = spec
        .data
        .as_ref()
        .and_then(|data| data.url.as_deref())
        .ok_or_else(|| Outcome::failed("the spec names no data: add `url` or `values`"))?;

    dataset_from(url, sandbox)
}

/// Load a table from a path inside the sandbox.
fn dataset_from(path: &str, sandbox: &Sandbox) -> Result<Dataset, Outcome> {
    let path = sandbox
        .read(path)
        .map_err(|error| Outcome::failed(error.to_string()))?;
    input::load(Some(&path))
        .map(|loaded| loaded.dataset)
        .map_err(|error| Outcome::failed(error.to_string()))
}

/// The catalogue entry for one mark.
fn mark_entry(mark: SpecMark, supported: bool) -> Value {
    let requirements = mark_requirements(mark);
    json!({
        "name": mark.as_str(),
        "namespace": if matches!(mark, SpecMark::Vega(_)) { "vega" } else { "charton" },
        "supported": supported,
        "required": requirements.required,
        "one_of": requirements.one_of,
        "forbidden": requirements.forbidden,
    })
}

/// The summary `profile_data` returns.
fn profile_of(loaded: &input::Loaded, sample_rows: usize) -> Value {
    let data = &loaded.dataset;
    let rows = data.height();

    let mut columns = Vec::new();
    for name in data.get_column_names() {
        let Ok(column) = data.column(&name) else {
            continue;
        };
        let missing = (0..rows).filter(|&row| column.is_null(row)).count();

        let mut entry = json!({
            "name": name,
            "type": semantic_name(column.semantic_type()),
            "missing": missing,
            "distinct": column.n_unique(),
        });

        match column.semantic_type() {
            charton::core::data::SemanticType::Continuous if missing < rows => {
                let (min, max) = column.min_max();
                entry["min"] = json!(min);
                entry["max"] = json!(max);
            }
            // Dates read better as the labels a chart would show than as
            // nanoseconds, so temporal columns are profiled like categories.
            _ => {
                let mut values = column.unique_values();
                values.truncate(20);
                entry["values"] = json!(values);
            }
        }

        columns.push(entry);
    }

    let sample: Vec<Value> = (0..rows.min(sample_rows))
        .map(|row| {
            let mut object = serde_json::Map::new();
            for name in data.get_column_names() {
                let value = data.get(&name, row);
                let cell = if data.is_null(&name, row) {
                    Value::Null
                } else if let Some(number) = value.to_f64() {
                    json!(number)
                } else {
                    json!(value.to_string().unwrap_or_default())
                };
                object.insert(name, cell);
            }
            Value::Object(object)
        })
        .collect();

    json!({
        "rows": rows,
        "columns": columns,
        "sample": sample,
    })
}

fn semantic_name(semantic: charton::core::data::SemanticType) -> &'static str {
    use charton::core::data::SemanticType;

    match semantic {
        SemanticType::Continuous => "continuous",
        SemanticType::Discrete => "categorical",
        SemanticType::Temporal => "temporal",
    }
}

/// A validation report as a tool outcome.
///
/// `failed` forces the error flag even for an empty report, which is what
/// rendering and exporting want: they have nothing to show when the document is
/// wrong, so the caller should stop rather than read a success.
fn report(errors: Vec<charton::spec::SpecError>, failed: bool) -> Outcome {
    let report = if errors.is_empty() {
        ErrorReport::ok()
    } else {
        ErrorReport::failed(errors)
    };
    let mut outcome = Outcome::json(&serde_json::to_value(&report).unwrap_or(Value::Null));
    outcome.is_error = failed || !report.ok;
    outcome
}
