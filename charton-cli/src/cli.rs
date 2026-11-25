//! The command line.
//!
//! Four commands, deliberately: `plot` for the common case, `render` for a
//! document, `validate` to check one, `schema` for editors and agents. Adding a
//! flag for every core capability is how a CLI turns into a second dialect — a
//! complex chart is written as a spec, not as a hundred switches.

use std::path::PathBuf;

use charton::spec::{ExtensionMark, MarkType};
use clap::{Args, Parser, Subcommand};

/// The command-line entry point.
#[derive(Debug, Parser)]
#[command(
    name = "charton",
    version,
    about = "Render charts from CSV, NDJSON or stdin — no Rust toolchain needed",
    long_about = "Charton speaks a subset of Vega-Lite v5. `plot` is the short path \
                  from a table to a chart; `render` takes a chart document; \
                  `validate` checks a document and explains what is wrong; \
                  `schema` prints the JSON Schema for editors and agents."
)]
pub struct Cli {
    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// The four commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Draw a chart from a table with a few flags.
    Plot(Box<PlotArgs>),
    /// Draw a chart from a JSON spec.
    Render(RenderArgs),
    /// Check a spec, and optionally its data.
    Validate(ValidateArgs),
    /// Print the JSON Schema for this build.
    Schema(SchemaArgs),
    /// Serve the Model Context Protocol tools over stdio.
    Mcp(Box<McpArgs>),
}

/// Where the rows come from.
#[derive(Debug, Args)]
pub struct InputArg {
    /// CSV or NDJSON file. Omit to read stdin.
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,
}

/// Where the image goes.
#[derive(Debug, Args)]
pub struct OutputArgs {
    /// Output file: .svg, .png or .pdf, or `-` for stdout.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Overwrite the output file if it exists.
    #[arg(long)]
    pub force: bool,
}

/// `charton plot`
#[derive(Debug, Args)]
pub struct PlotArgs {
    /// The rows to draw.
    #[command(flatten)]
    pub input: InputArg,

    /// The mark to draw.
    #[arg(long, value_name = "MARK", value_parser = mark_names(), default_value = "point")]
    pub mark: String,

    /// Column for the horizontal position.
    #[arg(long, value_name = "COLUMN")]
    pub x: Option<String>,

    /// Column for the vertical position.
    #[arg(long, value_name = "COLUMN")]
    pub y: Option<String>,

    /// Column for the far end of a vertical span.
    #[arg(long, value_name = "COLUMN")]
    pub y2: Option<String>,

    /// Column that drives colour.
    #[arg(long, value_name = "COLUMN")]
    pub color: Option<String>,

    /// Column that drives mark size.
    #[arg(long, value_name = "COLUMN")]
    pub size: Option<String>,

    /// Column that drives mark shape.
    #[arg(long, value_name = "COLUMN")]
    pub shape: Option<String>,

    /// Column that supplies label text.
    #[arg(long, value_name = "COLUMN")]
    pub text: Option<String>,

    /// Column that groups rows into separate lines.
    #[arg(long, value_name = "COLUMN")]
    pub path_group: Option<String>,

    /// Column holding the surface height, for `--mark contour`.
    #[arg(long, value_name = "COLUMN")]
    pub z: Option<String>,

    /// Chart title.
    #[arg(long)]
    pub title: Option<String>,

    /// Accessible description.
    #[arg(long)]
    pub description: Option<String>,

    /// Write the equivalent spec to this file (`-` for stdout).
    #[arg(long, value_name = "FILE")]
    pub emit_spec: Option<PathBuf>,

    /// Where the image goes.
    #[command(flatten)]
    pub output: OutputArgs,
}

/// `charton render`
#[derive(Debug, Args)]
pub struct RenderArgs {
    /// The spec to render.
    #[arg(value_name = "SPEC")]
    pub spec: PathBuf,

    /// Where the image goes.
    #[command(flatten)]
    pub output: OutputArgs,
}

/// `charton validate`
#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// The spec to check.
    #[arg(value_name = "SPEC")]
    pub spec: PathBuf,

    /// Also check the spec against these rows.
    #[arg(long, value_name = "DATA")]
    pub data: Option<PathBuf>,

    /// Print the report as JSON on stdout.
    #[arg(long)]
    pub json: bool,
}

/// `charton mcp`
#[derive(Debug, Args)]
pub struct McpArgs {
    /// Directory the tools may read from. Nothing outside it is reachable.
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub root: PathBuf,

    /// Allow `export_spec` to write files inside the root.
    #[arg(long)]
    pub allow_write: bool,
}

/// `charton schema`
#[derive(Debug, Args)]
pub struct SchemaArgs {
    /// Write the schema here instead of stdout.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Overwrite the output file if it exists.
    #[arg(long)]
    pub force: bool,
}

/// Every mark name `--mark` accepts: Vega-Lite's, then Charton's own.
fn mark_names() -> clap::builder::PossibleValuesParser {
    let names = MarkType::ALL
        .iter()
        .map(|mark| mark.as_str())
        .chain(ExtensionMark::ALL.iter().map(|mark| mark.as_str()));
    clap::builder::PossibleValuesParser::new(names)
}
