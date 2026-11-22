/// Converts a Polars `DataFrame` into a Charton [`Dataset`](crate::core::data::Dataset).
///
/// Designed for Polars 0.53 and later (tested against 0.53, 0.54, and 0.55).
/// For Polars 0.44 - 0.52 use `load_polars_v44_52!` instead.
///
/// This macro bridge facilitates the transition from Polars' analytical ecosystem
/// to Charton's visualization-ready data structures. It preserves the semantic
/// integrity of each column:
///
/// ### Mapping Logic:
/// - **Continuous**: Maps all Floating Point and Integer types (from `Int8` to `UInt64`).
///   `UInt8`/`UInt16` are widened losslessly to Charton's `UInt32` vector, and
///   `Float16` is widened to `Float32`.
/// - **Discrete**: Maps `String`, `Boolean`, and encoded `Categorical/Enum` types.
/// - **Temporal**: Maps `Date`, `Time`, `Duration`, and `Datetime`.
///   All temporal types are normalized to Charton's internal standard:
///   - `Datetime`: i64 nanoseconds since Unix Epoch.
///   - `Date`: i32 days since Unix Epoch.
///   - `Time`: i64 nanoseconds since midnight.
///   - `Duration`: i64 nanoseconds.
///
/// # Errors
/// Returns [`ChartonError::Data`](crate::error::ChartonError::Data) if a column contains a Polars type not yet
/// supported by Charton's core vectors (e.g., List, Struct, or Binary).
#[macro_export]
macro_rules! load_polars_df {
    ($df:expr) => {{
        let df = $df;
        let mut dataset: $crate::core::data::Dataset = $crate::core::data::Dataset::new();

        for column in df.columns() {
            // Convert Option<&Series> to Result<&Series, ChartonError>
            let series = column.as_series().ok_or_else(|| {
                $crate::error::ChartonError::Data(format!(
                    "Column '{}' is not a Series",
                    column.name()
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
                    let vec: Vec<Option<f64>> = ca.iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::Float32 => {
                    let ca = series.f32().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<f32>> = ca.iter().collect();
                    dataset.add_column(name, vec)?;
                }
                // Charton has no Float16 vector; widen losslessly to Float32.
                polars::prelude::DataType::Float16 => {
                    let casted = series
                        .cast(&polars::prelude::DataType::Float32)
                        .map_err(|e| {
                            $crate::error::ChartonError::Data(format!(
                                "Column '{}' cast error: {}",
                                name, e
                            ))
                        })?;
                    let ca = casted.f32().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<f32>> = ca.iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::Int64 => {
                    let ca = series.i64().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<i64>> = ca.iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::Int32 => {
                    let ca = series.i32().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<i32>> = ca.iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::Int16 => {
                    let ca = series.i16().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<i16>> = ca.iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::Int8 => {
                    let ca = series.i8().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<i8>> = ca.iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::UInt64 => {
                    let ca = series.u64().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<u64>> = ca.iter().collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::UInt32 => {
                    let ca = series.u32().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<u32>> = ca.iter().collect();
                    dataset.add_column(name, vec)?;
                }
                // Charton has no dedicated UInt8/UInt16 vector; widen losslessly.
                polars::prelude::DataType::UInt16 => {
                    let ca = series.u16().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<u32>> =
                        ca.iter().map(|opt| opt.map(u32::from)).collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::UInt8 => {
                    let ca = series.u8().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<u32>> =
                        ca.iter().map(|opt| opt.map(u32::from)).collect();
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
                        .iter()
                        .map(|opt| opt.map(|s| s.to_string()))
                        .collect();
                    dataset.add_column(name, vec)?;
                }

                polars::prelude::DataType::Categorical(_, _)
                | polars::prelude::DataType::Enum(_, _) => {
                    // Since Polars 0.53 the physical backing type of Categorical/Enum
                    // columns can be U8, U16 or U32, so we must dispatch on it instead
                    // of assuming a 32-bit key.
                    fn __charton_cat_to_column<T: polars::prelude::PolarsCategoricalType>(
                        ca: &polars::prelude::CategoricalChunked<T>,
                    ) -> $crate::core::data::ColumnVector {
                        $crate::core::data::ColumnVector::from_str_as_cat_opt(ca.iter_str())
                    }

                    let cv = if let Some(ca) = series.try_cat8() {
                        __charton_cat_to_column(ca)
                    } else if let Some(ca) = series.try_cat16() {
                        __charton_cat_to_column(ca)
                    } else if let Some(ca) = series.try_cat32() {
                        __charton_cat_to_column(ca)
                    } else {
                        return Err($crate::error::ChartonError::Data(format!(
                            "Column '{}' has an unsupported Categorical/Enum physical type: {:?}",
                            name,
                            series.dtype()
                        ))
                        .into());
                    };

                    dataset.add_column(name, cv)?;
                }

                polars::prelude::DataType::Boolean => {
                    let ca = series.bool().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<bool>> = ca.iter().collect();
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
                        .iter()
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
                        .iter()
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
                        .iter()
                        .map(|opt_nanos| {
                            opt_nanos.and_then(|n| {
                                // Polars stores Time as nanoseconds since midnight, while
                                // `from_hms_nano` expects a sub-second nanosecond component.
                                let secs = n.div_euclid(1_000_000_000);
                                let nanos = n.rem_euclid(1_000_000_000) as u32;
                                $crate::prelude::ctime::Time::from_hms_nano(
                                    (secs / 3600) as u8,
                                    ((secs / 60) % 60) as u8,
                                    (secs % 60) as u8,
                                    nanos,
                                )
                                .ok()
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
                        polars::prelude::TimeUnit::Milliseconds => 1_000_000,
                        polars::prelude::TimeUnit::Microseconds => 1_000,
                        polars::prelude::TimeUnit::Nanoseconds => 1,
                    };

                    let dur_vec: Vec<Option<$crate::prelude::ctime::Duration>> = ca
                        .physical()
                        .iter()
                        .map(|opt_v| {
                            opt_v.map(|v| {
                                let total_nanos = v.saturating_mul(multiplier);
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

        // Return a Result to allow the use of '?' in the calling context. Macros can't infer data types automatically.
        let res: std::result::Result<$crate::core::data::Dataset, $crate::error::ChartonError> =
            Ok(dataset);
        res
    }};
}

/// A convenience macro to initialize a [`Chart`](crate::chart::Chart) with data.
///
/// The `chart!` macro supports two primary usage patterns:
///
/// ### 1. Direct Variable Mapping (Auto-Stringify)
/// Pass one or more local variables. The macro will automatically use the
/// variable names as column names in the underlying [`Dataset`](crate::core::data::Dataset).
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
/// Pass a pre-constructed [`Dataset`](crate::core::data::Dataset) directly into the macro.
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
/// Returns [`ChartonError::Data`](crate::error::ChartonError::Data) if the provided variables have inconsistent
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

/// Converts a Polars `DataFrame` into a Charton [`Dataset`](crate::core::data::Dataset) for Polars v0.44-v0.52.
///
/// This macro is specifically designed for older Polars versions (0.44-0.52) where
/// the API differs from v0.53+. It uses `get_columns()` instead of `columns()` but
/// both return `&[Column]`, requiring the same `as_series()` extraction pattern.
///
/// ### Mapping Logic:
/// - **Continuous**: Maps all Floating Point and Integer types (from `Int8` to `UInt64`).
///   `UInt8`/`UInt16` are widened losslessly to Charton's `UInt32` vector.
/// - **Discrete**: Maps `String`, `Boolean`, and encoded `Categorical/Enum` types.
/// - **Temporal**: Maps `Date`, `Time`, `Duration`, and `Datetime`.
///   All temporal types are normalized to Charton's internal standard:
///   - `Datetime`: i64 nanoseconds since Unix Epoch.
///   - `Date`: i32 days since Unix Epoch.
///   - `Time`: i64 nanoseconds since midnight.
///   - `Duration`: i64 nanoseconds.
///
/// # Errors
/// Returns [`ChartonError::Data`](crate::error::ChartonError::Data) if a column contains a Polars type not yet
/// supported by Charton's core vectors (e.g., List, Struct, or Binary).
#[macro_export]
macro_rules! load_polars_v44_52 {
    ($df:expr) => {{
        let df = $df;
        let mut dataset: $crate::core::data::Dataset = $crate::core::data::Dataset::new();

        for column in df.get_columns() {
            // Convert Option<&Series> to Result<&Series, ChartonError>
            let series = column.as_series().ok_or_else(|| {
                $crate::error::ChartonError::Data(format!(
                    "Column '{}' is not a Series",
                    column.name()
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
                // Charton has no dedicated UInt8/UInt16 vector; widen losslessly.
                polars::prelude::DataType::UInt16 => {
                    let ca = series.u16().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<u32>> =
                        ca.into_iter().map(|opt| opt.map(u32::from)).collect();
                    dataset.add_column(name, vec)?;
                }
                polars::prelude::DataType::UInt8 => {
                    let ca = series.u8().map_err(|e| {
                        $crate::error::ChartonError::Data(format!(
                            "Column '{}' cast error: {}",
                            name, e
                        ))
                    })?;
                    let vec: Vec<Option<u32>> =
                        ca.into_iter().map(|opt| opt.map(u32::from)).collect();
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
                                // Polars stores Time as nanoseconds since midnight, while
                                // `from_hms_nano` expects a sub-second nanosecond component.
                                let secs = n.div_euclid(1_000_000_000);
                                let nanos = n.rem_euclid(1_000_000_000) as u32;
                                $crate::prelude::ctime::Time::from_hms_nano(
                                    (secs / 3600) as u8,
                                    ((secs / 60) % 60) as u8,
                                    (secs % 60) as u8,
                                    nanos,
                                )
                                .ok()
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
                        polars::prelude::TimeUnit::Milliseconds => 1_000_000,
                        polars::prelude::TimeUnit::Microseconds => 1_000,
                        polars::prelude::TimeUnit::Nanoseconds => 1,
                    };

                    let dur_vec: Vec<Option<$crate::prelude::ctime::Duration>> = ca
                        .physical()
                        .into_iter()
                        .map(|opt_v| {
                            opt_v.map(|v| {
                                let total_nanos = v.saturating_mul(multiplier);
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

        // Return a Result to allow the use of '?' in the calling context. Macros can't infer data types automatically.
        let res: std::result::Result<$crate::core::data::Dataset, $crate::error::ChartonError> =
            Ok(dataset);
        res
    }};
}
