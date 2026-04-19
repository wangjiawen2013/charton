#[cfg(feature = "bridge")]
use polars::error::PolarsError;
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
    /// Used for inconsistent lengths, empty datasets, or invalid types.
    #[error("Data error: {0}")]
    Data(String),

    /// Error related to mark definitions or configurations.
    #[error("Mark error: {0}")]
    Mark(String),

    /// Error related to encoding specifications.
    #[error("Encoding error: {0}")]
    Encoding(String),

    /// Formatting error during string formatting operations.
    #[error("formatting error: {0}")]
    Fmt(#[from] std::fmt::Error),

    /// Error related to scale definitions or operations.
    #[error("Scale error: {0}")]
    Scale(String),

    /// Error during rendering operations.
    #[error("Render error: {0}")]
    Render(String),

    /// I/O error from standard library operations.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Error related to SVG processing or usvg operations.
    #[error("svg/usvg error")]
    Svg,

    /// Error related to executable path validation.
    #[error("Executable path error: {0}")]
    ExecutablePath(String),

    /// Error for unimplemented features.
    #[error("Unimplemented feature: {0}")]
    Unimplemented(String),

    /// Error for internal logic errors.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Error from the Polars library.
    ///
    /// This variant automatically converts `PolarsError` instances into `ChartonError`.
    /// The `?` operator will automatically perform this conversion when working with
    /// Polars operations that can fail.
    #[cfg(feature = "bridge")]
    #[error("polars error: {0}")]
    Polars(#[from] PolarsError),
}
