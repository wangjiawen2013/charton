/// Converts a Polars [`DataFrame`] into a Charton [`Dataset`].
///
/// This macro iterates through the columns of the provided DataFrame and extracts
/// data into contiguous `Vec<Option<T>>` structures, making them ready for
/// Charton's internal processing.
///
/// # Behavior
/// - **Supported Types**: Processes `Float32`, `Float64`, `Int32`, `Int64`, and `String` columns.
/// - **Unsupported Types**: Columns with other data types (e.g., `Boolean`, `List`, `DateTime`)
///   are silently skipped in the current implementation.
/// - **Null Handling**: Preserves null values by mapping Polars series to `Vec<Option<T>>`.
///
/// # Arguments
/// * `$df` - An expression that evaluates to a `polars::prelude::DataFrame`.
///
/// # Returns
/// A `Result` where:
/// * `Ok(Dataset)`: A new dataset populated with the supported columns from the DataFrame.
/// * `Err(ChartonError)`: A crate-specific error, typically [`ChartonError::Data`] if
///   column casting fails, or other variants as defined in the bridge.
///
/// # Example
/// ```ignore
/// use polars::prelude::*;
/// use charton::load_polars_df;
///
/// let df = df! {
///     "x" => &[1.0_f32, 2.0_f32, 3.0_f32],
///     "y" => &["A", "B", "C"]
/// }?;
///
/// let dataset = load_polars_df!(df)?;
/// assert_eq!(dataset.width(), 2);
/// ```
///
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
