/// Converts a Polars [`DataFrame`] into a Charton [`Dataset`].
///
/// This macro iterates through the columns of the provided DataFrame and extracts
/// numeric data (`Float32` and `Float64`) into contiguous `Vec`s suitable for
/// high-performance processing or GPU uploads.
///
/// # Behavior
/// - **Supported Types**: Only `Float32` and `Float64` columns are processed.
/// - **Unsupported Types**: Columns with other data types (e.g., `Int32`, `Utf8`)
///   are silently skipped.
/// - **Null Handling**: Null values are excluded during collection via `into_no_null_iter()`.
///
/// # Arguments
/// * `$df` - An expression that evaluates to a `polars::prelude::DataFrame`.
///
/// # Returns
/// A `Result` containing:
/// * `Ok(Dataset)`: The constructed dataset if successful.
/// * `Err(Box<dyn std::error::Error>)`: An error box (currently always `Ok` unless
///   panic occurs in unwraps, but typed for future error handling expansion).
///
/// # Example
/// ```ignore
/// use polars::prelude::*;
///
/// let df = df! {
///     "x" => &[1.0_f32, 2.0_f32, 3.0_f32],
///     "y" => &[4.0_f64, 5.0_f64, 6.0_f64]
/// }?;
///
/// let dataset = load_polars_df!(df);
/// assert_eq!(dataset.width(), 2);
/// ```
// We rely on the user's environment having 'polars' available.
// This import is local to the generated block and resolves in the caller's context.
#[macro_export]
macro_rules! load_polars_df {
    ($df:expr) => {{
        let df = $df;
        let mut dataset: $crate::core::data::Dataset = $crate::core::data::Dataset::new();

        for series in df.columns() {
            let name = series.name().to_string();
            match series.dtype() {
                // --- Floating Point Types (Uses NaN for Nulls) ---
                polars::prelude::DataType::Float32 => {
                    let ca = series.f32().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<f32>> = ca.into_iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::Float64 => {
                    let ca = series.f64().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<f64>> = ca.into_iter().collect();
                    dataset.add_column(name, vec)?;
                }

                // --- Integer Types (Uses Bitmask for Nulls) ---
                polars::prelude::DataType::Int64 => {
                    let ca = series.i64().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<i64>> = ca.into_iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::Int32 => {
                    let ca = series.i32().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<i32>> = ca.into_iter().collect();
                    dataset.add_column(name, vec)?;
                }

                // --- String Type (Uses Bitmask for Nulls) ---
                polars::prelude::DataType::String => {
                    let ca = series.str().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    // Convert Polars &str to owned String for ColumnVector::String
                    let vec: Vec<Option<String>> = ca
                        .into_iter()
                        .map(|opt| opt.map(|s| s.to_string()))
                        .collect();
                    dataset.add_column(name, vec)?;
                }

                // --- Fallback ---
                _ => {
                    // Currently skipping other types (e.g., Boolean, List, DateTime)
                    // TODO: Implement DataType::Datetime mapping to OffsetDateTime if needed
                }
            }
        }

        // Return a Result to allow the use of '?' in the calling context
        // and resolve the "unused Result" warning.
        let res: std::result::Result<$crate::core::data::Dataset, $crate::error::ChartonError> =
            Ok(dataset);
        res
    }};
}
