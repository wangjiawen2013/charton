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
                polars::prelude::DataType::UInt32 => {
                    let ca = series.u32().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<u32>> = ca.into_iter().collect();
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

                // --- Temporal Type (Datetme)
                // Bridges Polars Datetime (i64 + TimeUnit) to time::OffsetDateTime.
                polars::prelude::DataType::Datetime(unit, _) => {
                    let ca = series.datetime().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' datetime cast error: {}",
                            name, e
                        ))
                    })?;

                    let mut dt_vec: Vec<Option<time::OffsetDateTime>> =
                        Vec::with_capacity(ca.len());

                    for opt_ts in ca.into_iter() {
                        let dt = opt_ts.and_then(|ts| {
                            // Map Polars unit to total nanoseconds since Unix Epoch
                            let nanos = match unit {
                                polars::prelude::TimeUnit::Milliseconds => (ts as i128) * 1_000_000,
                                polars::prelude::TimeUnit::Microseconds => (ts as i128) * 1_000,
                                polars::prelude::TimeUnit::Nanoseconds => ts as i128,
                            };

                            // Attempt to create the OffsetDateTime
                            time::OffsetDateTime::from_unix_timestamp_nanos(nanos).ok()
                        });
                        dt_vec.push(dt);
                    }
                    dataset.add_column(name, dt_vec)?;
                }

                // --- Fallback ---
                _ => {
                    // Currently skipping other types (e.g., Boolean, List)
                    // TODO: Implement DataType::List
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

/// A convenience macro to initialize a [`Chart`] with data.
///
/// The `chart!` macro supports two primary usage patterns:
///
/// ### 1. Direct Variable Mapping (Auto-Stringify)
/// Pass one or more local variables. The macro will automatically use the
/// variable names as column names in the underlying [`Dataset`].
///
/// ```ignore
/// let x = vec![1.0, 2.0, 3.0];
/// let y = vec![10.0, 20.0, 30.0];
///
/// // This creates a Dataset with columns "x" and "y"
/// chart!(x, y)?
///     .mark_point()?
///     .encode((alt::x("x"), alt::y("y")))?
///     .save("out.svg")?;
/// ```
///
/// ### 2. Existing Dataset
/// Pass a pre-constructed [`Dataset`] directly into the macro.
///
/// ```ignore
/// let ds = get_data_from_source()?;
/// chart!(ds)?
///     .mark_line()?
///     .encode((alt::x("x"), alt::y("y")))?
///     .save("out.svg")?;
/// ```
///
/// # Errors
/// Returns [`ChartonError::Data`] if the provided variables have inconsistent
/// row lengths when building a new dataset.
///
/// # Panics
/// The macro itself does not panic, but it propagates errors via the `?` operator.
#[macro_export]
macro_rules! chart {
    // --- MODE 1: Dataset Reference Mode ---
    // Specifically matches a reference to an existing Dataset: chart!(&ds)
    (&$ds:expr) => {
        $crate::chart::Chart::build($ds.clone())
    };

    // --- MODE 2: Dataset Ownership Move Mode ---
    // Specifically matches an owned Dataset: chart!(ds)
    ($ds:expr) => {
        $crate::chart::Chart::build($ds)
    };

    // --- MODE 3: Variadic Variable Mode ---
    // We use a specialized pattern to capture either '&ident' or 'ident'
    // without using 'tt', which prevents the "leftover tokens" issue.
    ($( $(&)? $col:ident ),+ $(,)?) => {{
        let mut ds = $crate::core::data::Dataset::new();
        let mut result = Ok(ds);

        $(
            // We pass the tokens exactly as matched to the internal parser
            result = $crate::chart!(@parse_col result, $col);
        )+

        result.and_then(|ds| $crate::chart::Chart::build(ds))
    }};

    // --- INTERNAL STRICT DISPATCHER ---

    // Sub-mode: Borrowed variable
    // This handles the logic when the caller provided '&variable'
    (@parse_col $res:ident, &$name:ident) => {
        $res.and_then(|mut d| {
            d.add_column(stringify!($name), $name.clone())?;
            Ok(d)
        })
    };

    // Sub-mode: Owned variable
    // This handles the logic when the caller provided 'variable'
    (@parse_col $res:ident, $name:ident) => {
        $res.and_then(|mut d| {
            d.add_column(stringify!($name), $name.clone())?;
            Ok(d)
        })
    };
}
