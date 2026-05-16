/// Converts a Polars [`DataFrame`] into a Charton [`Dataset`].
///
/// This macro bridge facilitates the transition from Polars' analytical ecosystem
/// to Charton's visualization-ready data structures. It preserves the semantic
/// integrity of each column:
///
/// ### Mapping Logic:
/// - **Continuous**: Maps all Floating Point and Integer types (from `Int8` to `UInt64`).
/// - **Discrete**: Maps `String`, `Boolean`, and encoded `Categorical/Enum` types.
/// - **Temporal**: Maps `Date`, `Time`, `Duration`, and `Datetime`.
///   All temporal types are normalized to Charton's internal standard:
///   - `Datetime`: i64 nanoseconds since Unix Epoch.
///   - `Date`: i32 days since Unix Epoch.
///   - `Time`: i64 nanoseconds since midnight.
///   - `Duration`: i64 nanoseconds.
///
/// # Errors
/// Returns [`ChartonError::Data`] if a column contains a Polars type not yet
/// supported by Charton's core vectors (e.g., List, Struct, or Binary).
#[macro_export]
macro_rules! load_polars_df {
    ($df:expr) => {{
        let df = $df;
        let mut dataset: $crate::core::data::Dataset = $crate::core::data::Dataset::new();

        for column in df.columns() {
            // Convert Result<Series, PolarsError> to Result<Series, ChartonError>
            let series = column.as_series().map_err(|e| {
                $crate::error::ChartonError::Data(format!(
                    "Failed to convert column to Series: {}",
                    e
                ))
            })?;
            
            let name = series.name().to_string();
            
            match series.dtype() {
                // --- Continuous: Numerical types ---
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
                polars::prelude::DataType::Int16 => {
                    let ca = series.i16().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<i16>> = ca.into_iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::Int8 => {
                    let ca = series.i8().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<i8>> = ca.into_iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::UInt64 => {
                    let ca = series.u64().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<u64>> = ca.into_iter().collect();
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

                // --- Discrete: Qualitative types ---
                polars::prelude::DataType::String => {
                    let ca = series.str().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<String>> = ca
                        .into_iter()
                        .map(|opt| opt.map(|s| s.to_string()))
                        .collect();
                    dataset.add_column(name, vec)?;
                }
                
                polars::prelude::DataType::Categorical(_, _)
                | polars::prelude::DataType::Enum(_, _) => {
                    let ca = series.cat32().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' categorical error: {}",
                            name, e
                        ))
                    })?;
                    let physical = ca.physical();

                    // Extract keys (indices)
                    let keys: Vec<u32> = physical.into_no_null_iter().collect();
                    
                    // Extract dictionary values
                    let values: Vec<String> = ca
                        .iter_str()
                        .map(|opt| opt.unwrap_or("").to_string())
                        .collect();

                    // Recover validity bitmask if the column contains nulls
                    let validity = if physical.null_count() > 0 {
                        physical
                            .chunks()
                            .get(0)
                            .and_then(|array| array.validity().map(|v| v.as_slice().0.to_vec()))
                    } else {
                        None
                    };

                    let cv = $crate::core::data::ColumnVector::from_categorical(keys, values, validity);
                    dataset.add_column(name, cv)?;
                }
                
                polars::prelude::DataType::Boolean => {
                    let ca = series.bool().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<bool>> = ca.into_iter().collect();
                    dataset.add_column(name, vec)?;
                }

                // --- Temporal: Normalized to Charton Standards ---
                
                polars::prelude::DataType::Datetime(unit, _tz) => {
                    let ca = series.datetime().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' datetime error: {}",
                            name, e
                        ))
                    })?;
                    
                    // Convert Polars physical timestamp to Chrono/Time compatible nanoseconds
                    let multiplier = match unit {
                        polars::prelude::TimeUnit::Milliseconds => 1_000_000i128,
                        polars::prelude::TimeUnit::Microseconds => 1_000i128,
                        polars::prelude::TimeUnit::Nanoseconds => 1i128,
                    };

                    let dt_vec: Vec<Option<$crate::prelude::ctime::OffsetDateTime>> = ca
                        .physical()
                        .into_iter()
                        .map(|opt_ts| {
                            opt_ts.and_then(|ts| {
                                let nanos = (ts as i128) * multiplier;
                                $crate::prelude::ctime::OffsetDateTime::from_unix_timestamp_nanos(nanos).ok()
                            })
                        })
                        .collect();
                    
                    // The From<Vec<Option<OffsetDateTime>>> impl in data.rs will 
                    // convert these to i64 nanoseconds internally.
                    dataset.add_column(name, dt_vec)?;
                }

                polars::prelude::DataType::Date => {
                    let ca = series.date().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' date error: {}",
                            name, e
                        ))
                    })?;
                    
                    let unix_epoch = $crate::prelude::ctime::Date::from_calendar_date(
                        1970,
                        $crate::prelude::ctime::Month::January,
                        1,
                    ).unwrap();

                    let date_vec: Vec<Option<$crate::prelude::ctime::Date>> = ca
                        .physical()
                        .into_iter()
                        .map(|opt_days| {
                            opt_days.and_then(|d| {
                                unix_epoch.checked_add($crate::prelude::ctime::Duration::days(d as i64))
                            })
                        })
                        .collect();
                        
                    // The From<Vec<Option<Date>>> impl in data.rs will 
                    // convert these to i32 days since epoch internally.
                    dataset.add_column(name, date_vec)?;
                }

                polars::prelude::DataType::Time => {
                    let ca = series.time().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' time error: {}",
                            name, e
                        ))
                    })?;
                    
                    let time_vec: Vec<Option<$crate::prelude::ctime::Time>> = ca
                        .physical()
                        .into_iter()
                        .map(|opt_nanos| {
                            opt_nanos.and_then(|n| {
                                // Polars Time is already in nanoseconds since midnight
                                $crate::prelude::ctime::Time::from_hms_nano(0, 0, 0, n as u32).ok()
                            })
                        })
                        .collect();
                        
                    // The From<Vec<Option<Time>>> impl in data.rs will 
                    // convert these to i64 nanoseconds since midnight internally.
                    dataset.add_column(name, time_vec)?;
                }

                polars::prelude::DataType::Duration(unit) => {
                    let ca = series.duration().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' duration error: {}",
                            name, e
                        ))
                    })?;
                    
                    let multiplier = match unit {
                        polars::prelude::TimeUnit::Milliseconds => 1_000_000i128,
                        polars::prelude::TimeUnit::Microseconds => 1_000i128,
                        polars::prelude::TimeUnit::Nanoseconds => 1i128,
                    };

                    let dur_vec: Vec<Option<$crate::prelude::ctime::Duration>> = ca
                        .physical()
                        .into_iter()
                        .map(|opt_v| {
                            opt_v.map(|v| {
                                let total_nanos = (v as i128) * multiplier;
                                $crate::prelude::ctime::Duration::nanoseconds(total_nanos)
                            })
                        })
                        .collect();
                        
                    // The From<Vec<Option<Duration>>> impl in data.rs will 
                    // convert these to i64 nanoseconds internally.
                    dataset.add_column(name, dur_vec)?;
                }

                _ => {
                    return Err($crate::error::ChartonError::Data(format!(
                        "Unsupported Polars DataType '{:?}' in column '{}'.",
                        series.dtype(),
                        name
                    ))
                    .into());
                }
            }
        }
        Ok(dataset)
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

// Polars v0.42-v0.52 specific data ingestion macro.
//
// This version is tailored for older Polars APIs (using `get_columns()`) and
// maps data to Charton's Categorical, Continuous, and Temporal semantic types.