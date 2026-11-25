//! Writing the rendered bytes.
//!
//! The format comes from the extension, or stdout for `-`. An existing file is
//! kept unless `--force` is given. Nothing here writes a log line, so `-o -`
//! pipes an image cleanly.
use std::path::{Path, PathBuf};

use charton::core::composite::LayeredChart;

use crate::errors::CliError;

/// Default output when `--output` is not given.
const DEFAULT_OUTPUT: &str = "chart.svg";

/// An image format, decided by the extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Vector, always available.
    Svg,
    /// Raster.
    Png,
    /// Vector, paginated.
    Pdf,
}

impl Format {
    fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "svg" => Some(Self::Svg),
            "png" => Some(Self::Png),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }
}

/// Where the image goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// A file on disk.
    File {
        /// The path to write.
        path: PathBuf,
        /// The format its extension asks for.
        format: Format,
    },
    /// Standard output, which must be an SVG: a terminal is not a PDF viewer.
    Stdout,
}

/// Decide where to write, from the flag and the extension.
///
/// # Errors
///
/// Fails when the extension is not one we render, or when the file exists and
/// `--force` was not given.
pub fn resolve(output: Option<&Path>, force: bool) -> Result<Destination, CliError> {
    let Some(output) = output else {
        return resolve(Some(Path::new(DEFAULT_OUTPUT)), force);
    };

    if output == Path::new("-") {
        return Ok(Destination::Stdout);
    }

    let format = Format::from_path(output).ok_or_else(|| {
        CliError::usage(format!(
            "cannot tell the format of {}; use .svg, .png or .pdf",
            output.display()
        ))
    })?;

    if output.exists() && !force {
        return Err(CliError::usage(format!(
            "{} already exists; pass --force to overwrite it",
            output.display()
        )));
    }

    Ok(Destination::File {
        path: output.to_owned(),
        format,
    })
}

/// Render the chart and write it.
///
/// # Errors
///
/// Fails when rendering or writing fails.
pub fn write_chart(
    chart: &LayeredChart,
    output: Option<&Path>,
    force: bool,
) -> Result<Destination, CliError> {
    let destination = resolve(output, force)?;
    write_bytes(&render(chart, &destination)?, &destination)?;
    Ok(destination)
}

/// Render the chart in the destination's format.
fn render(chart: &LayeredChart, destination: &Destination) -> Result<Vec<u8>, CliError> {
    match destination {
        Destination::Stdout => chart
            .to_svg()
            .map(String::into_bytes)
            .map_err(|error| CliError::render(format!("cannot render the chart: {error}"))),
        Destination::File { format, .. } => match format {
            Format::Svg => chart
                .to_svg()
                .map(String::into_bytes)
                .map_err(|error| CliError::render(format!("cannot render the chart: {error}"))),
            Format::Png => chart
                .to_png()
                .map_err(|error| CliError::render(format!("cannot render the chart: {error}"))),
            Format::Pdf => chart
                .to_pdf()
                .map_err(|error| CliError::render(format!("cannot render the chart: {error}"))),
        },
    }
}

/// Write bytes to the destination.
fn write_bytes(bytes: &[u8], destination: &Destination) -> Result<(), CliError> {
    match destination {
        Destination::Stdout => {
            use std::io::Write as _;
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(bytes)
                .and_then(|()| stdout.flush())
                .map_err(|error| CliError::render(format!("cannot write stdout: {error}")))
        }
        Destination::File { path, .. } => std::fs::write(path, bytes)
            .map_err(|error| CliError::render(format!("cannot write {}: {error}", path.display()))),
    }
}

/// Write a text file, honouring `--force` and `-` for stdout.
///
/// Used by `--emit-spec` and `schema`, which produce documents rather than
/// images but follow the same overwrite rules.
///
/// # Errors
///
/// Fails when the file exists without `--force`, or cannot be written.
pub fn write_text(text: &str, output: Option<&Path>, force: bool) -> Result<(), CliError> {
    let Some(output) = output else {
        print!("{text}");
        return Ok(());
    };

    if output == Path::new("-") {
        print!("{text}");
        return Ok(());
    }

    if output.exists() && !force {
        return Err(CliError::usage(format!(
            "{} already exists; pass --force to overwrite it",
            output.display()
        )));
    }

    std::fs::write(output, text)
        .map_err(|error| CliError::render(format!("cannot write {}: {error}", output.display())))
}
