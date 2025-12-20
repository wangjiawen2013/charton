use polars::prelude::PolarsError;
use thiserror::Error;

/// The main error type for the charton crate.
///
/// This enum encompasses all possible error conditions that can occur within
/// the charton library. It uses the `thiserror` crate to provide automatic
/// implementation of `std::error::Error` and `Display` traits, making error
/// handling consistent and ergonomic.
///
/// Most variants wrap a `String` message that describes the specific error,
/// while some variants automatically convert from underlying library errors
/// using the `#[from]` attribute.
#[derive(Error, Debug)]
pub enum ChartonError {
    /// Error related to data handling or processing.
    ///
    /// This variant covers issues such as invalid data formats, missing data,
    /// or data that cannot be processed for visualization.
    #[error("Data error: {0}")]
    Data(String),

    /// Error related to mark definitions or configurations.
    ///
    /// This variant covers issues with plot marks such as invalid mark types,
    /// unsupported configurations, or malformed mark specifications.
    #[error("Mark error: {0}")]
    Mark(String),

    /// Error related to encoding specifications.
    ///
    /// This variant covers issues with how data is mapped to visual properties,
    /// such as invalid column names, mismatched data types, or unsupported encodings.
    #[error("Encoding error: {0}")]
    Encoding(String),

    /// Error from the Polars library.
    ///
    /// This variant automatically converts `PolarsError` instances into `ChartonError`.
    /// The `?` operator will automatically perform this conversion when working with
    /// Polars operations that can fail.
    #[error("polars error: {0}")]
    Polars(#[from] PolarsError),

    /// Formatting error during string formatting operations.
    ///
    /// This variant automatically converts `std::fmt::Error` instances into `ChartonError`.
    #[error("formatting error: {0}")]
    Fmt(#[from] std::fmt::Error),

    /// Error related to scale definitions or operations.
    ///
    /// This variant covers issues with axis scales such as unsupported scale types,
    /// invalid scale parameters, or incompatible data for a given scale.
    #[error("Scale error: {0}")]
    Scale(String),

    /// Error during rendering operations.
    ///
    /// This variant covers issues that occur during the rendering process,
    /// such as SVG generation failures, unsupported render targets, or failures
    /// in external rendering engines. This includes:
    /// - Failures in external visualization libraries (Altair, Matplotlib)
    /// - SVG generation and processing errors
    /// - Template rendering failures
    /// - Image encoding/decoding problems
    #[error("Render error: {0}")]
    Render(String),

    /// I/O error from standard library operations.
    ///
    /// This variant automatically converts `std::io::Error` instances into `ChartonError`.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Error related to SVG processing or usvg operations.
    ///
    /// This variant covers issues with SVG handling, including parsing or
    /// processing errors with the usvg backend.
    #[error("svg/usvg error")]
    Svg,

    /// Error related to executable path validation.
    ///
    /// This variant covers issues with interpreter or executable paths such as:
    /// - Path does not exist
    /// - Path points to a directory instead of a file
    /// - File is not executable
    /// - Insufficient permissions to access the executable
    #[error("Executable path error: {0}")]
    ExecutablePath(String),

    /// Error for unimplemented features.
    ///
    /// This variant is used when a requested feature or functionality has not
    /// yet been implemented in the library.
    #[error("Unimplemented feature: {0}")]
    Unimplemented(String),

    /// Error for internal logic errors.
    ///
    /// This variant is used when an unexpected condition occurs in the internal
    /// logic of the library, typically indicating a bug or inconsistency in the code.
    /// These errors should not normally occur during regular usage but might happen
    /// due to edge cases or programming errors.
    #[error("Internal error: {0}")]
    Internal(String),
}
