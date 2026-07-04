use crate::error::ChartonError;
use ahash::{AHashMap, AHashSet};
use std::fmt;
use std::sync::Arc;
use time::{Date, Duration, OffsetDateTime, Time};

/// Encapsulates a single column of data with a high-performance memory layout.
///
/// Naming and structure are designed to be "Polars-friendly", allowing near
/// zero-cost conversion from Polars DataFrames while maintaining a
/// visualization-optimized architecture.
///
/// For temporal types, this enum leverages the Anchored Offset Mapping (ARTM)
/// philosophy, storing high-precision integers that are later normalized
/// against a reference anchor during rendering.
#[derive(Clone, Debug)]
pub enum ColumnVector {
    /// Boolean values (true/false). Nulls are tracked via a validity bitmask.
    Boolean {
        data: Vec<bool>,
        validity: Option<Vec<u8>>,
    },

    // --- Integer Types ---
    // Retained for memory efficiency in Wasm and zero-copy Polars compatibility.
    /// 8-bit signed integers.
    Int8 {
        data: Vec<i8>,
        validity: Option<Vec<u8>>,
    },
    /// 16-bit signed integers.
    Int16 {
        data: Vec<i16>,
        validity: Option<Vec<u8>>,
    },
    /// 32-bit signed integers.
    Int32 {
        data: Vec<i32>,
        validity: Option<Vec<u8>>,
    },
    /// 64-bit signed integers.
    Int64 {
        data: Vec<i64>,
        validity: Option<Vec<u8>>,
    },
    /// 32-bit unsigned integers. Often used for indexing or Categorical keys.
    UInt32 {
        data: Vec<u32>,
        validity: Option<Vec<u8>>,
    },
    /// 64-bit unsigned integers. Often used for large IDs or hashes.
    UInt64 {
        data: Vec<u64>,
        validity: Option<Vec<u8>>,
    },

    // --- Floating Point Types ---
    /// 32-bit floating point numbers.
    Float32 {
        data: Vec<f32>,
        validity: Option<Vec<u8>>,
    },
    /// 64-bit floating point numbers. The primary type for coordinate calculations.
    Float64 {
        data: Vec<f64>,
        validity: Option<Vec<u8>>,
    },

    // --- String & Categorical Types ---
    /// UTF-8 String data. Best for low-cardinality metadata (e.g., tooltips).
    String {
        data: Vec<String>,
        validity: Option<Vec<u8>>,
    },
    /// Categorical data using an index-to-dictionary mapping.
    /// Perfectly maps to Polars' `Categorical` or `Enum` types.
    Categorical {
        /// Physical indices pointing into the values vector.
        keys: Vec<u32>,
        /// The dictionary of unique string labels.
        values: Vec<String>,
        validity: Option<Vec<u8>>,
    },

    // --- Temporal Types ---
    // Stored as physical primitives (i32/i64) to ensure SIMD-friendly scaling.
    /// Date representing a calendar date (Year, Month, Day).
    ///
    /// Stored as an i32 representing the number of days since the UNIX Epoch (1970-01-01).
    /// Matches Polars' `Date` and supports efficient calendar arithmetic.
    Date {
        data: Vec<i32>,
        validity: Option<Vec<u8>>,
    },

    /// Datetime representing a specific point in time.
    ///
    /// Stored as an i64 representing elapsed time since the UNIX Epoch.
    Datetime {
        data: Vec<i64>,
        validity: Option<Vec<u8>>,
        /// Optional IANA timezone string (e.g., "UTC", "Asia/Singapore").
        /// Essential for correct label formatting on axes.
        timezone: Option<String>,
    },

    /// Duration representing a time span or interval.
    ///
    /// Stored as an i64.
    Duration {
        data: Vec<i64>,
        validity: Option<Vec<u8>>,
    },

    /// Time representing a specific time of day, independent of any date.
    ///
    /// Stored as an i64 representing the offset since midnight (00:00:00).
    Time {
        data: Vec<i64>,
        validity: Option<Vec<u8>>,
    },
}

/// Mapping raw types to semantic types allows the engine to automatically
/// select the appropriate Scale (Linear, Temporal, or Discrete) and validation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    /// Quantitative/Numeric data that supports arithmetic and interpolation (e.g., 1.2, 100).
    /// Maps to: LinearScale, LogScale, SqrtScale.
    Continuous,

    /// Categorical or Qualitative data used for grouping or indexing (e.g., "Apple", "Orange").
    /// Maps to: DiscreteScale (Ordinal or Nominal).
    Discrete,

    /// Time-based data.
    Temporal,
}

/// Represents a dynamically-typed scalar value borrowed safely from a ColumnVector.
/// This acts as the single unified interface for extracting row-level data.
#[derive(Debug, Clone, PartialEq)]
pub enum AnyValue<'a> {
    Null,
    Boolean(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    String(&'a str),
    Date(i32),
    Datetime(i64, Option<&'a str>),
    Duration(i64),
    Time(i64),
}

impl<'a> AnyValue<'a> {
    /// Safely casts any numeric or temporal type to f64.
    /// Handles NaN filtering and boolean mapping automatically.
    #[inline]
    pub const fn to_f64(&self) -> Option<f64> {
        match self {
            AnyValue::Null => None,
            AnyValue::Boolean(v) => Some(if *v { 1.0 } else { 0.0 }),
            AnyValue::Int8(v) => Some(*v as f64),
            AnyValue::Int16(v) => Some(*v as f64),
            AnyValue::Int32(v) => Some(*v as f64),
            AnyValue::Int64(v) => Some(*v as f64),
            AnyValue::UInt32(v) => Some(*v as f64),
            AnyValue::UInt64(v) => Some(*v as f64),
            AnyValue::Float32(v) => {
                if v.is_nan() {
                    None
                } else {
                    Some(*v as f64)
                }
            }
            AnyValue::Float64(v) => {
                if v.is_nan() {
                    None
                } else {
                    Some(*v)
                }
            }
            AnyValue::Date(v) => Some(*v as f64),
            AnyValue::Datetime(v, _) => Some(*v as f64),
            AnyValue::Duration(v) => Some(*v as f64),
            AnyValue::Time(v) => Some(*v as f64),
            AnyValue::String(_) => None,
        }
    }

    /// Formats the scalar value into an owned String for display/tooltips.
    pub fn to_string(&self) -> Option<String> {
        match self {
            AnyValue::Null => None,
            AnyValue::Boolean(v) => Some(v.to_string()),
            AnyValue::Int8(v) => Some(v.to_string()),
            AnyValue::Int16(v) => Some(v.to_string()),
            AnyValue::Int32(v) => Some(v.to_string()),
            AnyValue::Int64(v) => Some(v.to_string()),
            AnyValue::UInt32(v) => Some(v.to_string()),
            AnyValue::UInt64(v) => Some(v.to_string()),
            AnyValue::Float32(v) => {
                if v.is_nan() {
                    None
                } else {
                    Some(v.to_string())
                }
            }
            AnyValue::Float64(v) => {
                if v.is_nan() {
                    None
                } else {
                    Some(v.to_string())
                }
            }
            AnyValue::String(s) => Some(s.to_string()),
            AnyValue::Date(v) => Some(v.to_string()),
            AnyValue::Datetime(v, _) => Some(v.to_string()),
            AnyValue::Duration(v) => Some(v.to_string()),
            AnyValue::Time(v) => Some(v.to_string()),
        }
    }
}

impl ColumnVector {
    /// Creates a Categorical column from pre-encoded keys and a dictionary.
    ///
    /// This is the preferred method for bridges (like Polars) where data is
    /// already physically separated into indices and values.
    pub const fn from_categorical(
        keys: Vec<u32>,
        values: Vec<String>,
        validity: Option<Vec<u8>>,
    ) -> Self {
        Self::Categorical {
            keys,
            values,
            validity,
        }
    }

    /// Creates a `Categorical` column from a sequence of strings.
    ///
    /// This method automatically handles:
    /// 1. Dictionary encoding (deduplicating strings into the `values` vector).
    /// 2. Null tracking (generating the `validity` bitmask if `None` is present).
    /// 3. Physical mapping (assigning `u32` keys to each entry).
    pub fn from_str_as_cat_opt<S, I>(iter: I) -> Self
    where
        S: AsRef<str>,
        I: IntoIterator<Item = Option<S>>,
    {
        let mut values = Vec::new();
        let mut lookup = std::collections::HashMap::new();
        let mut keys = Vec::new();

        let mut validity_builder = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_idx = 0;
        let mut has_null = false;

        for item in iter {
            // Expand validity bitmask buffer
            if bit_idx == 8 {
                validity_builder.push(current_byte);
                current_byte = 0;
                bit_idx = 0;
            }

            match item {
                Some(s) => {
                    let s_ref = s.as_ref();
                    // Get existing key or create new one
                    let &mut key = lookup.entry(s_ref.to_string()).or_insert_with(|| {
                        let new_key = values.len() as u32;
                        values.push(s_ref.to_string());
                        new_key
                    });
                    keys.push(key);
                    // Mark as valid (1)
                    current_byte |= 1 << bit_idx;
                }
                None => {
                    keys.push(0); // Placeholder for null
                    has_null = true;
                    // Mark as invalid (0)
                }
            }
            bit_idx += 1;
        }

        // Push trailing validity bits
        if bit_idx > 0 {
            validity_builder.push(current_byte);
        }

        Self::Categorical {
            keys,
            values,
            validity: if has_null {
                Some(validity_builder)
            } else {
                None
            },
        }
    }

    /// no null version: support &str / String / &String
    pub fn from_str_as_cat<S, I>(iter: I) -> Self
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S>,
    {
        Self::from_str_as_cat_opt(iter.into_iter().map(Some))
    }

    /// Infers the [SemanticType] of the column based on its internal storage variant.
    ///
    /// This is a low-latency operation used to guide the selection of
    /// visual encoding strategies (e.g., choosing a TemporalScale for Datetime).
    pub const fn semantic_type(&self) -> SemanticType {
        match self {
            // --- Continuous: Measurable numeric values ---
            ColumnVector::Float64 { .. }
            | ColumnVector::Float32 { .. }
            | ColumnVector::Int64 { .. }
            | ColumnVector::Int32 { .. }
            | ColumnVector::Int16 { .. }
            | ColumnVector::Int8 { .. }
            | ColumnVector::UInt64 { .. }
            | ColumnVector::UInt32 { .. } => SemanticType::Continuous,

            // --- Discrete: Qualitative categories ---
            ColumnVector::String { .. }
            | ColumnVector::Categorical { .. }
            | ColumnVector::Boolean { .. } => SemanticType::Discrete,

            // --- Temporal: Points or spans in time ---
            ColumnVector::Date { .. }
            | ColumnVector::Datetime { .. }
            | ColumnVector::Time { .. }
            | ColumnVector::Duration { .. } => SemanticType::Temporal,
        }
    }

    /// Returns a short string representation of the data type,
    /// consistent with Polars' naming conventions (e.g., "f64", "str", "datetime").
    ///
    /// This is used primarily for diagnostic printing and debugging,
    /// allowing users to quickly identify the physical storage of a column.
    pub const fn dtype_name(&self) -> &'static str {
        match self {
            // --- Floats ---
            ColumnVector::Float64 { .. } => "f64",
            ColumnVector::Float32 { .. } => "f32",

            // --- Signed Integers ---
            ColumnVector::Int64 { .. } => "i64",
            ColumnVector::Int32 { .. } => "i32",
            ColumnVector::Int16 { .. } => "i16",
            ColumnVector::Int8 { .. } => "i8",

            // --- Unsigned Integers ---
            ColumnVector::UInt64 { .. } => "u64",
            ColumnVector::UInt32 { .. } => "u32",

            // --- Booleans ---
            ColumnVector::Boolean { .. } => "bool",

            // --- Strings & Categorical ---
            ColumnVector::String { .. } => "str", // Polars uses "str" for String/Utf8
            ColumnVector::Categorical { .. } => "cat", // Consistent with Polars' Categorical shorthand

            // --- Temporal: Simplified Polars-style naming ---
            ColumnVector::Date { .. } => "date",

            // For Datetime, Time, and Duration, Polars usually clarifies the unit.
            // Since we return &'static str, we pick the most descriptive shorthand.
            ColumnVector::Datetime { .. } => "datetime[ns]",
            ColumnVector::Duration { .. } => "duration[ns]",
            ColumnVector::Time { .. } => "time[ns]",
        }
    }

    /// Returns the number of rows in this column.
    pub const fn len(&self) -> usize {
        match self {
            // Standard numeric and boolean types
            ColumnVector::Boolean { data, .. } => data.len(),
            ColumnVector::Int8 { data, .. } => data.len(),
            ColumnVector::Int16 { data, .. } => data.len(),
            ColumnVector::Int32 { data, .. } => data.len(),
            ColumnVector::Int64 { data, .. } => data.len(),
            ColumnVector::UInt32 { data, .. } => data.len(),
            ColumnVector::UInt64 { data, .. } => data.len(),
            ColumnVector::Float32 { data, .. } => data.len(),
            ColumnVector::Float64 { data, .. } => data.len(),

            // Strings
            ColumnVector::String { data, .. } => data.len(),

            // Categorical: The length is determined by the number of keys (indices),
            // not the number of unique values in the dictionary.
            ColumnVector::Categorical { keys, .. } => keys.len(),

            // Temporal types
            ColumnVector::Date { data, .. } => data.len(),
            ColumnVector::Datetime { data, .. } => data.len(),
            ColumnVector::Duration { data, .. } => data.len(),
            ColumnVector::Time { data, .. } => data.len(),
        }
    }

    /// Returns `true` if the column contains no elements.
    ///
    /// This is the preferred way to check for empty columns in Rust
    /// as it aligns with standard library collection APIs.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Checks if a specific row index is marked as valid in the optional bitmask.
    ///
    /// - If the mask is `None`, all rows are considered valid.
    /// - If the mask exists, it performs a bitwise check: (byte >> bit_offset) & 1.
    pub fn is_valid_in_mask(mask: &Option<Vec<u8>>, index: usize) -> bool {
        match mask {
            // No mask means data is 100% complete.
            None => true,
            Some(bits) => {
                let byte_idx = index / 8;
                let bit_idx = index % 8;

                // Get the specific byte, then check the bit at bit_idx.
                // We return false if the index is somehow out of bounds.
                bits.get(byte_idx)
                    .map(|&byte| (byte >> bit_idx) & 1 == 1)
                    .unwrap_or(false)
            }
        }
    }

    /// Single source of truth for retrieving cell data.
    pub fn get(&self, row: usize) -> AnyValue<'_> {
        if row >= self.len() || self.is_null(row) {
            return AnyValue::Null;
        }

        match self {
            ColumnVector::Boolean { data, .. } => AnyValue::Boolean(data[row]),
            ColumnVector::Int8 { data, .. } => AnyValue::Int8(data[row]),
            ColumnVector::Int16 { data, .. } => AnyValue::Int16(data[row]),
            ColumnVector::Int32 { data, .. } => AnyValue::Int32(data[row]),
            ColumnVector::Int64 { data, .. } => AnyValue::Int64(data[row]),
            ColumnVector::UInt32 { data, .. } => AnyValue::UInt32(data[row]),
            ColumnVector::UInt64 { data, .. } => AnyValue::UInt64(data[row]),
            ColumnVector::Float32 { data, .. } => AnyValue::Float32(data[row]),
            ColumnVector::Float64 { data, .. } => AnyValue::Float64(data[row]),
            ColumnVector::String { data, .. } => AnyValue::String(&data[row]),
            ColumnVector::Categorical { keys, values, .. } => {
                let key = keys[row] as usize;
                AnyValue::String(&values[key])
            }
            ColumnVector::Date { data, .. } => AnyValue::Date(data[row]),
            ColumnVector::Datetime { data, timezone, .. } => {
                AnyValue::Datetime(data[row], timezone.as_deref())
            }
            ColumnVector::Duration { data, .. } => AnyValue::Duration(data[row]),
            ColumnVector::Time { data, .. } => AnyValue::Time(data[row]),
        }
    }

    /// [WASM PREPARATION]: High-performance in-place capacity reuse.
    /// Clears the existing f64 vector and copies new data without reallocating heap memory.
    pub fn update_f64_data(&mut self, new_data: &[f64]) -> Result<(), ChartonError> {
        match self {
            ColumnVector::Float64 { data, .. } => {
                data.clear();
                data.extend_from_slice(new_data); // Extremely fast memcpy under the hood
                Ok(())
            }
            _ => Err(ChartonError::Data(
                "Cannot perform in-place f64 update on non-Float64 column".to_string(),
            )),
        }
    }

    /// Projects the entire column into a contiguous `f64` vector.
    ///
    /// This is a high-cost operation ($O(n)$ time + memory allocation).
    /// To maximize performance, the type-check is hoisted outside the loop,
    /// allowing the compiler to optimize the inner loops for specific data types
    /// and enabling SIMD auto-vectorization.
    pub fn to_f64_vec(&self) -> Vec<f64> {
        let n = self.len();
        let mut out = Vec::with_capacity(n);

        match self {
            // --- Floating Point Types ---
            // These require an explicit NaN check to ensure the output buffer
            // is safe for rendering (NaNs can cause issues in coordinate systems).
            ColumnVector::Float64 { data, validity } => {
                for (i, &v) in data.iter().enumerate().take(n) {
                    let is_valid = Self::is_valid_in_mask(validity, i);
                    out.push(if is_valid && !v.is_nan() { v } else { 0.0 });
                }
            }
            ColumnVector::Float32 { data, validity } => {
                for (i, &v) in data.iter().enumerate().take(n) {
                    let is_valid = Self::is_valid_in_mask(validity, i);
                    out.push(if is_valid && !v.is_nan() {
                        v as f64
                    } else {
                        0.0
                    });
                }
            }

            // --- 64-bit Integer & Temporal Types ---
            // Datetime, Duration, and Time are physically stored as i64.
            // We project the raw integer values into f64 for numerical calculations.
            ColumnVector::Int64 { data, validity }
            | ColumnVector::Datetime { data, validity, .. }
            | ColumnVector::Duration { data, validity, .. }
            | ColumnVector::Time { data, validity, .. } => {
                for (i, &v) in data.iter().enumerate().take(n) {
                    out.push(if Self::is_valid_in_mask(validity, i) {
                        v as f64
                    } else {
                        0.0
                    });
                }
            }

            // --- 32-bit Integer & Date Types ---
            // Date is physically stored as an i32 representing days since the Epoch.
            ColumnVector::Int32 { data, validity } | ColumnVector::Date { data, validity } => {
                for (i, &v) in data.iter().enumerate().take(n) {
                    out.push(if Self::is_valid_in_mask(validity, i) {
                        v as f64
                    } else {
                        0.0
                    });
                }
            }

            // --- Unsigned Integer Types ---
            // Handled separately to accommodate different underlying data types in the enum variants.
            ColumnVector::UInt32 { data, validity } => {
                for (i, &v) in data.iter().enumerate().take(n) {
                    out.push(if Self::is_valid_in_mask(validity, i) {
                        v as f64
                    } else {
                        0.0
                    });
                }
            }
            ColumnVector::UInt64 { data, validity } => {
                for (i, &v) in data.iter().enumerate().take(n) {
                    out.push(if Self::is_valid_in_mask(validity, i) {
                        v as f64
                    } else {
                        0.0
                    });
                }
            }

            // --- Small Integer Types ---
            ColumnVector::Int16 { data, validity } => {
                for (i, &v) in data.iter().enumerate().take(n) {
                    out.push(if Self::is_valid_in_mask(validity, i) {
                        v as f64
                    } else {
                        0.0
                    });
                }
            }
            ColumnVector::Int8 { data, validity } => {
                for (i, &v) in data.iter().enumerate().take(n) {
                    out.push(if Self::is_valid_in_mask(validity, i) {
                        v as f64
                    } else {
                        0.0
                    });
                }
            }

            // --- Boolean Type ---
            // Logical mapping where true becomes 1.0 and false becomes 0.0.
            ColumnVector::Boolean { data, validity } => {
                for (i, &v) in data.iter().enumerate().take(n) {
                    out.push(if Self::is_valid_in_mask(validity, i) && v {
                        1.0
                    } else {
                        0.0
                    });
                }
            }

            // --- Non-Numeric Types ---
            // Strings and Categorical data cannot be projected to a continuous numerical scale.
            // We return a zero-filled vector of length `n` as a safe fallback.
            ColumnVector::String { .. } | ColumnVector::Categorical { .. } => {
                out.resize(n, 0.0);
            }
        }

        out
    }

    /// Projects the column into a vector of `Option<f64>`, preserving the original
    /// null/validity states. Useful for statistical calculations where nulls
    /// should not be coerced to 0.0.
    pub fn to_f64_options(&self) -> Vec<Option<f64>> {
        (0..self.len()).map(|i| self.get(i).to_f64()).collect()
    }

    /// Creates a new ColumnVector containing only the specified rows based on the provided indices.
    ///
    /// This is a fundamental operation for filtering, sorting, and shuffling.
    /// It preserves the original variant type and re-indexes the validity
    /// bitmask to ensure null-state consistency after row reordering.
    pub fn take(&self, indices: &[usize]) -> Self {
        match self {
            // --- Floating Point Types ---
            ColumnVector::Float64 { data, validity } => ColumnVector::Float64 {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },
            ColumnVector::Float32 { data, validity } => ColumnVector::Float32 {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },

            // --- Integer Types ---
            ColumnVector::Int64 { data, validity } => ColumnVector::Int64 {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },
            ColumnVector::Int32 { data, validity } => ColumnVector::Int32 {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },
            ColumnVector::Int16 { data, validity } => ColumnVector::Int16 {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },
            ColumnVector::Int8 { data, validity } => ColumnVector::Int8 {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },
            ColumnVector::UInt64 { data, validity } => ColumnVector::UInt64 {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },
            ColumnVector::UInt32 { data, validity } => ColumnVector::UInt32 {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },

            // --- Boolean Type ---
            ColumnVector::Boolean { data, validity } => ColumnVector::Boolean {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },

            // --- Strings & Categorical Types ---
            ColumnVector::String { data, validity } => ColumnVector::String {
                data: indices.iter().map(|&i| data[i].clone()).collect(),
                validity: self.take_validity(validity, indices),
            },
            ColumnVector::Categorical {
                keys,
                values,
                validity,
            } => ColumnVector::Categorical {
                keys: indices.iter().map(|&i| keys[i]).collect(),
                values: values.clone(), // The dictionary/mapping is preserved
                validity: self.take_validity(validity, indices),
            },

            // --- Temporal Types ---
            ColumnVector::Date { data, validity } => ColumnVector::Date {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },
            ColumnVector::Datetime {
                data,
                validity,
                timezone,
            } => ColumnVector::Datetime {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
                timezone: timezone.clone(),
            },
            ColumnVector::Duration { data, validity } => ColumnVector::Duration {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },
            ColumnVector::Time { data, validity } => ColumnVector::Time {
                data: indices.iter().map(|&i| data[i]).collect(),
                validity: self.take_validity(validity, indices),
            },
        }
    }

    /// Re-indexes the packed bitmask (validity map) based on the provided row indices.
    ///
    /// This function creates a new bitmask where each bit represents the validity
    /// of the row at that position in the new index set. If the input validity
    /// is None, it implies all rows are valid, and None is returned.
    fn take_validity(&self, validity: &Option<Vec<u8>>, indices: &[usize]) -> Option<Vec<u8>> {
        // If the original column has no nulls (None), the new column won't either.
        validity.as_ref()?;

        let num_rows = indices.len();
        // Allocate a zeroed bitmask. 0 means Null/Invalid by default.
        let mut new_mask = vec![0u8; num_rows.div_ceil(8)];

        for (new_idx, &old_idx) in indices.iter().enumerate() {
            // Check if the original row was valid using the helper method.
            if Self::is_valid_in_mask(validity, old_idx) {
                let byte_idx = new_idx / 8;
                let bit_idx = new_idx % 8;
                // Set the corresponding bit to 1 (Valid).
                new_mask[byte_idx] |= 1 << bit_idx;
            }
        }

        Some(new_mask)
    }

    /// Returns true if the value at the given row is considered "null".
    ///
    /// This method is the "source of truth" for data presence. It checks:
    /// 1. The validity bitmask (if present, a 0 bit indicates a Null value).
    /// 2. Floating-point NaN (Not-a-Number) status, which is also treated as Null.
    pub fn is_null(&self, row: usize) -> bool {
        match self {
            // Floating-point types require a dual check: the bitmask AND NaN status.
            ColumnVector::Float64 { data, validity } => {
                !Self::is_valid_in_mask(validity, row) || data[row].is_nan()
            }
            ColumnVector::Float32 { data, validity } => {
                !Self::is_valid_in_mask(validity, row) || data[row].is_nan()
            }

            // For all other types (Integers, Strings, Categorical, Temporal),
            // Null status is determined solely by the validity bitmask.
            _ => !Self::is_valid_in_mask(self.get_validity_mask(), row),
        }
    }

    /// Provides a reference to the optional validity bitmask for this column.
    ///
    /// The bitmask uses a packed `u8` format where each bit represents the
    /// validity of a row (1 for Valid, 0 for Null).
    pub const fn get_validity_mask(&self) -> &Option<Vec<u8>> {
        match self {
            ColumnVector::Float64 { validity, .. } => validity,
            ColumnVector::Float32 { validity, .. } => validity,
            ColumnVector::Int8 { validity, .. } => validity,
            ColumnVector::Int16 { validity, .. } => validity,
            ColumnVector::Int32 { validity, .. } => validity,
            ColumnVector::Int64 { validity, .. } => validity,
            ColumnVector::UInt32 { validity, .. } => validity,
            ColumnVector::UInt64 { validity, .. } => validity,
            ColumnVector::Boolean { validity, .. } => validity,
            ColumnVector::String { validity, .. } => validity,
            ColumnVector::Categorical { validity, .. } => validity,
            ColumnVector::Date { validity, .. } => validity,
            ColumnVector::Datetime { validity, .. } => validity,
            ColumnVector::Time { validity, .. } => validity,
            ColumnVector::Duration { validity, .. } => validity,
        }
    }

    /// Returns the number of unique non-null values in the column.
    ///
    /// The 'Null' state is determined strictly by the validity bitmask.
    /// For floating-point types, NaNs are treated as valid unique values if
    /// the bitmask marks the row as valid.
    pub fn n_unique(&self) -> usize {
        // --- FAST PATH: Categorical ---
        if let ColumnVector::Categorical { values, .. } = self {
            return values.len();
        }

        #[cfg(feature = "parallel")]
        {
            use ahash::AHashSet;
            use rayon::prelude::*;

            macro_rules! parallel_unique_impl {
                ($data:expr, $validity:expr) => {
                    (0..$data.len())
                        .into_par_iter()
                        .fold(AHashSet::new, |mut set, i| {
                            // Bitmask is the source of truth for Null
                            if Self::is_valid_in_mask($validity, i) {
                                set.insert($data[i].clone());
                            }
                            set
                        })
                        .reduce(AHashSet::new, |mut s1, s2| {
                            s1.extend(s2);
                            s1
                        })
                        .len()
                };
            }

            match self {
                ColumnVector::Categorical { .. } => unreachable!(),

                // --- FLOAT PATHS ---
                // Even though we use validity masks, we still normalize -0.0 and 0.0
                // so they are not counted as two different unique values.
                ColumnVector::Float64 { data, validity } => (0..data.len())
                    .into_par_iter()
                    .fold(AHashSet::new, |mut set, i| {
                        if Self::is_valid_in_mask(validity, i) {
                            let v = data[i];
                            let norm = if v == 0.0 { 0.0 } else { v };
                            set.insert(norm.to_bits());
                        }
                        set
                    })
                    .reduce(AHashSet::new, |mut s1, s2| {
                        s1.extend(s2);
                        s1
                    })
                    .len(),

                ColumnVector::Float32 { data, validity } => (0..data.len())
                    .into_par_iter()
                    .fold(AHashSet::new, |mut set, i| {
                        if Self::is_valid_in_mask(validity, i) {
                            let v = data[i];
                            let norm = if v == 0.0 { 0.0 } else { v };
                            set.insert(norm.to_bits());
                        }
                        set
                    })
                    .reduce(AHashSet::new, |mut s1, s2| {
                        s1.extend(s2);
                        s1
                    })
                    .len(),

                // --- ALL OTHER PATHS ---
                ColumnVector::String { data, validity } => parallel_unique_impl!(data, validity),
                ColumnVector::Int64 { data, validity } => parallel_unique_impl!(data, validity),
                ColumnVector::Int32 { data, validity } => parallel_unique_impl!(data, validity),
                ColumnVector::Int16 { data, validity } => parallel_unique_impl!(data, validity),
                ColumnVector::Int8 { data, validity } => parallel_unique_impl!(data, validity),
                ColumnVector::UInt64 { data, validity } => parallel_unique_impl!(data, validity),
                ColumnVector::UInt32 { data, validity } => parallel_unique_impl!(data, validity),
                ColumnVector::Boolean { data, validity } => parallel_unique_impl!(data, validity),
                ColumnVector::Date { data, validity } => parallel_unique_impl!(data, validity),
                ColumnVector::Datetime { data, validity, .. } => {
                    parallel_unique_impl!(data, validity)
                }
                ColumnVector::Duration { data, validity, .. } => {
                    parallel_unique_impl!(data, validity)
                }
                ColumnVector::Time { data, validity, .. } => parallel_unique_impl!(data, validity),
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.n_unique_serial()
        }
    }

    /// Returns the number of unique non-null values using a single-threaded implementation.
    pub fn n_unique_serial(&self) -> usize {
        if let ColumnVector::Categorical { values, .. } = self {
            return values.len();
        }

        use ahash::AHashSet;

        macro_rules! serial_unique_impl {
            ($data:expr, $validity:expr) => {{
                let mut seen = AHashSet::with_capacity(($data.len() / 4).max(10));
                for (i, v) in $data.iter().enumerate() {
                    if Self::is_valid_in_mask($validity, i) {
                        seen.insert(v);
                    }
                }
                seen.len()
            }};
        }

        match self {
            ColumnVector::Categorical { .. } => unreachable!(),

            ColumnVector::Float64 { data, validity } => {
                let mut seen = AHashSet::with_capacity(data.len() / 4);
                for (i, &v) in data.iter().enumerate() {
                    if Self::is_valid_in_mask(validity, i) {
                        let norm = if v == 0.0 { 0.0 } else { v };
                        seen.insert(norm.to_bits());
                    }
                }
                seen.len()
            }
            ColumnVector::Float32 { data, validity } => {
                let mut seen = AHashSet::with_capacity(data.len() / 4);
                for (i, &v) in data.iter().enumerate() {
                    if Self::is_valid_in_mask(validity, i) {
                        let norm = if v == 0.0 { 0.0 } else { v };
                        seen.insert(norm.to_bits());
                    }
                }
                seen.len()
            }

            ColumnVector::String { data, validity } => serial_unique_impl!(data, validity),
            ColumnVector::Int64 { data, validity } => serial_unique_impl!(data, validity),
            ColumnVector::Int32 { data, validity } => serial_unique_impl!(data, validity),
            ColumnVector::Int16 { data, validity } => serial_unique_impl!(data, validity),
            ColumnVector::Int8 { data, validity } => serial_unique_impl!(data, validity),
            ColumnVector::UInt64 { data, validity } => serial_unique_impl!(data, validity),
            ColumnVector::UInt32 { data, validity } => serial_unique_impl!(data, validity),
            ColumnVector::Boolean { data, validity } => serial_unique_impl!(data, validity),
            ColumnVector::Date { data, validity } => serial_unique_impl!(data, validity),
            ColumnVector::Datetime { data, validity, .. } => serial_unique_impl!(data, validity),
            ColumnVector::Duration { data, validity, .. } => serial_unique_impl!(data, validity),
            ColumnVector::Time { data, validity, .. } => serial_unique_impl!(data, validity),
        }
    }

    /// Returns a stable, unique list of values as Strings for Discrete scales.
    ///
    /// The 'Null' state is determined strictly by the validity bitmask.
    /// This method preserves the "First Appearance" order to ensure stable
    /// visual mapping in charts (e.g., consistent color assignment).
    pub fn unique_values(&self) -> Vec<String> {
        // --- FAST PATH: Categorical ---
        if let ColumnVector::Categorical { values, .. } = self {
            return values.clone();
        }

        let mut result = Vec::new();

        match self {
            ColumnVector::Categorical { .. } => unreachable!(),

            // --- FLOAT PATHS ---
            // We trust the validity mask. NaNs are included if valid.
            // We normalize -0.0 and 0.0 to prevent duplicate string entries.
            ColumnVector::Float64 { data, validity } => {
                let mut seen = AHashSet::new();
                for (i, &v) in data.iter().enumerate() {
                    if Self::is_valid_in_mask(validity, i) {
                        let norm = if v == 0.0 { 0.0 } else { v };
                        if seen.insert(norm.to_bits()) {
                            result.push(v.to_string());
                        }
                    }
                }
            }
            ColumnVector::Float32 { data, validity } => {
                let mut seen = AHashSet::new();
                for (i, &v) in data.iter().enumerate() {
                    if Self::is_valid_in_mask(validity, i) {
                        let norm = if v == 0.0 { 0.0 } else { v };
                        if seen.insert(norm.to_bits()) {
                            result.push(v.to_string());
                        }
                    }
                }
            }

            // --- STRING PATH ---
            ColumnVector::String { data, validity } => {
                let mut seen = AHashSet::new();
                for (i, s) in data.iter().enumerate() {
                    if Self::is_valid_in_mask(validity, i) && seen.insert(s) {
                        result.push(s.clone());
                    }
                }
            }

            // --- PRIMITIVE & TEMPORAL PATHS ---
            _ => {
                self.collect_unique_primitives_as_strings(&mut result);
            }
        }
        result
    }

    /// Internal helper that uses i128 casting to deduplicate various integer and
    /// temporal types into a single stable String vector.
    fn collect_unique_primitives_as_strings(&self, result: &mut Vec<String>) {
        let mut seen = AHashSet::<i128>::new();

        macro_rules! collect_cast {
            ($data:expr, $validity:expr) => {
                for (i, &v) in $data.iter().enumerate() {
                    if Self::is_valid_in_mask($validity, i) && seen.insert(v as i128) {
                        result.push(v.to_string());
                    }
                }
            };
        }

        match self {
            ColumnVector::Int8 { data, validity } => collect_cast!(data, validity),
            ColumnVector::Int16 { data, validity } => collect_cast!(data, validity),
            ColumnVector::Int32 { data, validity } => collect_cast!(data, validity),
            ColumnVector::Int64 { data, validity } => collect_cast!(data, validity),
            ColumnVector::UInt64 { data, validity } => collect_cast!(data, validity),
            ColumnVector::UInt32 { data, validity } => collect_cast!(data, validity),

            ColumnVector::Date { data, validity } => collect_cast!(data, validity),
            ColumnVector::Datetime { data, validity, .. } => collect_cast!(data, validity),
            ColumnVector::Duration { data, validity, .. } => collect_cast!(data, validity),
            ColumnVector::Time { data, validity, .. } => collect_cast!(data, validity),

            ColumnVector::Boolean { data, validity } => {
                for (i, &v) in data.iter().enumerate() {
                    if Self::is_valid_in_mask(validity, i) && seen.insert(if v { 1 } else { 0 }) {
                        result.push(v.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    /// Computes both minimum and maximum values in a single parallel scan.
    ///
    /// Returns a tuple `(min, max)` as `f64`. This method respects the
    /// validity bitmask and filters out floating-point NaNs for accurate ranges.
    pub fn min_max(&self) -> (f64, f64) {
        #[cfg(feature = "parallel")]
        {
            match self {
                // --- FLOATING POINT PATHS ---
                ColumnVector::Float64 { data, validity } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v)
                }
                ColumnVector::Float32 { data, validity } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }

                // --- INTEGER PATHS ---
                ColumnVector::Int64 { data, validity } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }
                ColumnVector::Int32 { data, validity } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }
                ColumnVector::Int16 { data, validity } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }
                ColumnVector::Int8 { data, validity } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }
                ColumnVector::UInt64 { data, validity } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }
                ColumnVector::UInt32 { data, validity } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }

                // --- TEMPORAL PATHS ---
                ColumnVector::Date { data, validity } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }
                ColumnVector::Datetime { data, validity, .. } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }
                ColumnVector::Duration { data, validity, .. } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }
                ColumnVector::Time { data, validity, .. } => {
                    self.parallel_scan_with_mask(data, validity, |&v| v as f64)
                }

                // --- DISCRETE/OTHER ---
                _ => (0.0, 0.0),
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.min_max_serial()
        }
    }

    /// Internal parallel scanner utilizing a Map-Reduce pattern for maximum throughput.
    #[cfg(feature = "parallel")]
    fn parallel_scan_with_mask<T, F>(
        &self,
        data: &[T],
        validity: &Option<Vec<u8>>,
        convert: F,
    ) -> (f64, f64)
    where
        T: Copy + Sync + Send,
        F: Fn(&T) -> f64 + Sync + Send,
    {
        use rayon::prelude::*;

        let identity = (f64::INFINITY, f64::NEG_INFINITY);

        data.par_iter()
            .enumerate()
            .fold(
                || identity,
                |(min, max), (i, v)| {
                    if Self::is_valid_in_mask(validity, i) {
                        let val = convert(v);
                        // CRITICAL: NaNs must be ignored for comparison to work.
                        if !val.is_nan() {
                            return (min.min(val), max.max(val));
                        }
                    }
                    (min, max)
                },
            )
            .reduce(|| identity, |(m1, x1), (m2, x2)| (m1.min(m2), x1.max(x2)))
    }

    /// Serial implementation of min_max.
    #[cfg(not(feature = "parallel"))]
    fn min_max_serial(&self) -> (f64, f64) {
        match self {
            ColumnVector::Float64 { data, validity } => {
                self.serial_scan_with_mask(data, validity, |&v| v)
            }
            ColumnVector::Float32 { data, validity } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::Int64 { data, validity } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::Int32 { data, validity } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::Int16 { data, validity } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::Int8 { data, validity } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::UInt64 { data, validity } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::UInt32 { data, validity } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::Date { data, validity } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::Datetime { data, validity, .. } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::Duration { data, validity, .. } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::Time { data, validity, .. } => {
                self.serial_scan_with_mask(data, validity, |&v| v as f64)
            }
            _ => (0.0, 0.0),
        }
    }

    /// Serial version of the mask scanner.
    #[cfg(not(feature = "parallel"))]
    fn serial_scan_with_mask<T, F>(
        &self,
        data: &[T],
        validity: &Option<Vec<u8>>,
        convert: F,
    ) -> (f64, f64)
    where
        F: Fn(&T) -> f64,
    {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for (i, v) in data.iter().enumerate() {
            if Self::is_valid_in_mask(validity, i) {
                let val = convert(v);
                if !val.is_nan() {
                    // Using f64::min/max is safer and more consistent with parallel path
                    min = min.min(val);
                    max = max.max(val);
                }
            }
        }
        (min, max)
    }

    /// Converts an Apache Arrow Array into a Charton ColumnVector.
    #[cfg(feature = "arrow")]
    pub fn from_arrow(array: &dyn Array) -> Result<Self, ChartonError> {
        use arrow::array::*;
        use arrow::datatypes::*;

        match array.data_type() {
            // --- FLOATING POINT ---
            DataType::Float64 => {
                let arr = array.as_primitive::<Float64Type>();
                let (data, validity) = Self::extract_arrow_primitives(arr);
                Ok(ColumnVector::Float64 { data, validity })
            }
            DataType::Float32 => {
                let arr = array.as_primitive::<Float32Type>();
                let (data, validity) = Self::extract_arrow_primitives(arr);
                Ok(ColumnVector::Float32 { data, validity })
            }

            // --- INTEGERS ---
            DataType::Int64 => {
                let arr = array.as_primitive::<Int64Type>();
                let (data, validity) = Self::extract_arrow_primitives(arr);
                Ok(ColumnVector::Int64 { data, validity })
            }
            DataType::Int32 => {
                let arr = array.as_primitive::<Int32Type>();
                let (data, validity) = Self::extract_arrow_primitives(arr);
                Ok(ColumnVector::Int32 { data, validity })
            }
            DataType::UInt32 => {
                let arr = array.as_primitive::<UInt32Type>();
                let (data, validity) = Self::extract_arrow_primitives(arr);
                Ok(ColumnVector::UInt32 { data, validity })
            }
            DataType::UInt64 => {
                let arr = array.as_primitive::<UInt64Type>();
                let (data, validity) = Self::extract_arrow_primitives(arr);
                Ok(ColumnVector::UInt64 { data, validity })
            }

            // --- STRINGS ---
            DataType::Utf8 | DataType::LargeUtf8 => {
                // Using as_string::<i32> or as_string::<i64> via AsArray trait
                let arr = array
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .or_else(|| array.as_any().downcast_ref::<LargeStringArray>())
                    .ok_or_else(|| {
                        ChartonError::Data("Failed to downcast string array".to_string())
                    })?;

                let (data, validity) = collect_with_validity(
                    (0..arr.len()).map(|i| {
                        if arr.is_valid(i) {
                            Some(arr.value(i).to_string())
                        } else {
                            None
                        }
                    }),
                    String::new(),
                );
                Ok(ColumnVector::String { data, validity })
            }

            // --- TEMPORAL & DURATION ---
            DataType::Timestamp(unit, _) => {
                let arr = array.as_primitive::<TimestampMicrosecondType>(); // This is tricky, see note below
                // Note: Arrow stores specific types for units. Better to use as_primitive_opt
                // or a generic helper. For simplicity, we'll use i64 physical access:
                let (data, validity) = Self::extract_arrow_i64_physical(array);
                Ok(ColumnVector::Datetime {
                    data,
                    validity,
                    unit: unit.clone().into(),
                })
            }
            DataType::Date32 => {
                let arr = array.as_primitive::<Date32Type>();
                let (data, validity) = Self::extract_arrow_primitives(arr);
                Ok(ColumnVector::Date { data, validity })
            }
            DataType::Duration(unit) => {
                let (data, validity) = Self::extract_arrow_i64_physical(array);
                Ok(ColumnVector::Duration {
                    data,
                    validity,
                    unit: unit.clone().into(),
                })
            }
            // Support for Time (Time32 uses i32, Time64 uses i64)
            DataType::Time32(unit) => {
                let arr = array.as_primitive::<Int32Type>(); // Physical view
                let (data, validity) = Self::extract_arrow_primitives(arr);
                // Cast i32 to i64 to match our ColumnVector::Time storage
                let data_i64 = data.into_iter().map(|v| v as i64).collect();
                Ok(ColumnVector::Time {
                    data: data_i64,
                    validity,
                    unit: unit.clone().into(),
                })
            }
            DataType::Time64(unit) => {
                let (data, validity) = Self::extract_arrow_i64_physical(array);
                Ok(ColumnVector::Time {
                    data,
                    validity,
                    unit: unit.clone().into(),
                })
            }

            _ => Err(ChartonError::Data(format!(
                "Unsupported Arrow type: {:?}",
                array.data_type()
            ))),
        }
    }

    /// Helper to extract data from any array that is physically i64 (Timestamp, Duration, Int64, Time64).
    #[cfg(feature = "arrow")]
    fn extract_arrow_i64_physical(array: &dyn Array) -> (Vec<i64>, Option<Vec<u8>>) {
        use arrow::array::AsArray;
        let arr = array.as_primitive::<Int64Type>();
        let data = arr.values().to_vec();
        let validity = arr.nulls().map(|nb| nb.buffer().as_slice().to_vec());
        (data, validity)
    }

    #[cfg(feature = "arrow")]
    fn extract_arrow_primitives<T>(arr: &PrimitiveArray<T>) -> (Vec<T::Native>, Option<Vec<u8>>)
    where
        T: arrow::datatypes::ArrowPrimitiveType,
    {
        let data = arr.values().to_vec();
        let validity = arr.nulls().map(|nb| nb.buffer().as_slice().to_vec());
        (data, validity)
    }

    /// Creates a new ColumnVector containing a sub-range of the data.
    ///
    /// This follows Charton's columnar layout: slicing owned data for eager operations.
    /// The validity bitmask (if present) is sliced and bit-shifted to maintain alignment.
    pub fn slice(&self, offset: usize, len: usize) -> Self {
        // Safety: Prevent out-of-bounds panics by saturating the range based on actual length.
        let total_len = self.len();
        let actual_offset = offset.min(total_len);
        let end = (actual_offset + len).min(total_len);
        let actual_len = end - actual_offset;

        match self {
            // --- Boolean Variant ---
            ColumnVector::Boolean { data, validity } => ColumnVector::Boolean {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },

            // --- Integer Variants ---
            ColumnVector::Int8 { data, validity } => ColumnVector::Int8 {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
            ColumnVector::Int16 { data, validity } => ColumnVector::Int16 {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
            ColumnVector::Int32 { data, validity } => ColumnVector::Int32 {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
            ColumnVector::Int64 { data, validity } => ColumnVector::Int64 {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
            ColumnVector::UInt32 { data, validity } => ColumnVector::UInt32 {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
            ColumnVector::UInt64 { data, validity } => ColumnVector::UInt64 {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },

            // --- Floating Point Variants ---
            ColumnVector::Float32 { data, validity } => ColumnVector::Float32 {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
            ColumnVector::Float64 { data, validity } => ColumnVector::Float64 {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },

            // --- String & Categorical Variants ---
            ColumnVector::String { data, validity } => ColumnVector::String {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
            ColumnVector::Categorical {
                keys,
                values,
                validity,
            } => ColumnVector::Categorical {
                keys: keys[actual_offset..end].to_vec(),
                values: values.clone(), // Dictionary remains shared
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },

            // --- Temporal Variants ---
            ColumnVector::Date { data, validity } => ColumnVector::Date {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
            ColumnVector::Time { data, validity } => ColumnVector::Time {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
            ColumnVector::Datetime {
                data,
                validity,
                timezone,
            } => ColumnVector::Datetime {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
                timezone: timezone.clone(),
            },
            ColumnVector::Duration { data, validity } => ColumnVector::Duration {
                data: data[actual_offset..end].to_vec(),
                validity: validity
                    .as_ref()
                    .map(|v| self.slice_validity(v, actual_offset, actual_len)),
            },
        }
    }

    /// Slices a validity bitmap [u8] by accounting for bit-level offsets.
    ///
    /// Since the 'offset' might not be a multiple of 8, we cannot simply slice the bytes.
    /// This function performs bit-shifting to realign the bits so that the new bitmap
    /// starts at bit 0 for the first row of the sliced data.
    fn slice_validity(&self, v: &[u8], offset: usize, len: usize) -> Vec<u8> {
        if len == 0 {
            return Vec::new();
        }

        let n_bytes = len.div_ceil(8);
        let mut new_v = Vec::with_capacity(n_bytes);

        let byte_offset = offset / 8;
        let bit_shift = offset % 8;

        if bit_shift == 0 {
            // OPTIMIZATION: If the offset is byte-aligned, perform a direct slice copy.
            // We use .min() and .resize() to safely handle cases where the source buffer
            // is not padded to full bytes.
            let end_byte = (byte_offset + n_bytes).min(v.len());
            new_v.extend_from_slice(&v[byte_offset..end_byte]);

            if new_v.len() < n_bytes {
                new_v.resize(n_bytes, 0);
            }
        } else {
            // RE-ALIGNMENT: Stitch adjacent bytes together using bitwise shifting.
            // We assume LSB-first (Least Significant Bit) bit order, matching Arrow standards.
            for i in 0..n_bytes {
                // SAFETY: Using .get().unwrap_or(0) prevents panics if the source bitmap
                // is shorter than the requested slice range.
                let current = v.get(byte_offset + i).copied().unwrap_or(0);
                let next = v.get(byte_offset + i + 1).copied().unwrap_or(0);

                // Construct the new byte by combining the high bits of the current byte
                // with the low bits of the subsequent byte.
                let combined = (current >> bit_shift) | (next << (8 - bit_shift));
                new_v.push(combined);
            }
        }

        // HYGIENE: The last byte might contain "dirty" bits beyond the requested 'len'.
        // We apply a bitmask to zero out bits exceeding the length to ensure consistent
        // results in downstream operations like population counts or equality checks.
        if let Some(last) = new_v.last_mut() {
            let trailing_bits = len % 8;
            if trailing_bits != 0 {
                let mask = (1u8 << trailing_bits) - 1;
                *last &= mask;
            }
        }

        new_v
    }
}

// --- Float Variants (Now with Validity Bitmask support) ---
impl From<Vec<Option<f64>>> for ColumnVector {
    fn from(v: Vec<Option<f64>>) -> Self {
        let (data, validity) = collect_with_validity(v, f64::NAN);
        ColumnVector::Float64 { data, validity }
    }
}

impl From<Vec<Option<f32>>> for ColumnVector {
    fn from(v: Vec<Option<f32>>) -> Self {
        let (data, validity) = collect_with_validity(v, f32::NAN);
        ColumnVector::Float32 { data, validity }
    }
}

// --- Integer Variants ---
impl From<Vec<Option<i64>>> for ColumnVector {
    fn from(v: Vec<Option<i64>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0i64);
        ColumnVector::Int64 { data, validity }
    }
}

impl From<Vec<Option<i32>>> for ColumnVector {
    fn from(v: Vec<Option<i32>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0i32);
        ColumnVector::Int32 { data, validity }
    }
}

impl From<Vec<Option<i16>>> for ColumnVector {
    fn from(v: Vec<Option<i16>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0i16);
        ColumnVector::Int16 { data, validity }
    }
}

impl From<Vec<Option<i8>>> for ColumnVector {
    fn from(v: Vec<Option<i8>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0i8);
        ColumnVector::Int8 { data, validity }
    }
}

impl From<Vec<Option<u64>>> for ColumnVector {
    fn from(v: Vec<Option<u64>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0u64);
        ColumnVector::UInt64 { data, validity }
    }
}

impl From<Vec<Option<u32>>> for ColumnVector {
    fn from(v: Vec<Option<u32>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0u32);
        ColumnVector::UInt32 { data, validity }
    }
}

// --- Boolean Variant ---
impl From<Vec<Option<bool>>> for ColumnVector {
    fn from(v: Vec<Option<bool>>) -> Self {
        let (data, validity) = collect_with_validity(v, false);
        ColumnVector::Boolean { data, validity }
    }
}

// --- String Variants ---
impl From<Vec<Option<String>>> for ColumnVector {
    fn from(v: Vec<Option<String>>) -> Self {
        let (data, validity) = collect_with_validity(v, String::new());
        ColumnVector::String { data, validity }
    }
}

impl From<Vec<Option<&str>>> for ColumnVector {
    fn from(v: Vec<Option<&str>>) -> Self {
        let (data, validity) = collect_with_validity(
            v.into_iter().map(|opt| opt.map(|s| s.to_string())),
            String::new(),
        );
        ColumnVector::String { data, validity }
    }
}

// --- Temporal Variant Implementations ---

impl From<Vec<Option<Date>>> for ColumnVector {
    /// Maps Date objects to i32 days relative to Unix Epoch (1970-01-01).
    /// Physical unit: Day (inferred).
    fn from(v: Vec<Option<Date>>) -> Self {
        let unix_epoch = Date::from_calendar_date(1970, time::Month::January, 1).unwrap();
        let mapped = v
            .into_iter()
            .map(|opt| opt.map(|d| (d - unix_epoch).whole_days() as i32));

        let (data, validity) = collect_with_validity(mapped, 0i32);
        ColumnVector::Date { data, validity }
    }
}

impl From<Vec<Option<OffsetDateTime>>> for ColumnVector {
    /// Projects OffsetDateTime objects into i64 nanoseconds.
    /// Retains the maximum precision provided by the input objects.
    fn from(v: Vec<Option<OffsetDateTime>>) -> Self {
        let mapped = v
            .into_iter()
            .map(|opt| opt.map(|dt| dt.unix_timestamp_nanos() as i64));

        let (data, validity) = collect_with_validity(mapped, 0i64);
        ColumnVector::Datetime {
            data,
            validity,
            timezone: None,
        }
    }
}

impl From<Vec<Option<Time>>> for ColumnVector {
    /// Maps Time objects to i64 nanoseconds elapsed since 00:00:00.
    fn from(v: Vec<Option<Time>>) -> Self {
        let mapped = v
            .into_iter()
            .map(|opt| opt.map(|t| (t - Time::MIDNIGHT).whole_nanoseconds() as i64));

        let (data, validity) = collect_with_validity(mapped, 0i64);
        ColumnVector::Time { data, validity }
    }
}

impl From<Vec<Option<Duration>>> for ColumnVector {
    fn from(v: Vec<Option<Duration>>) -> Self {
        let mapped = v
            .into_iter()
            .map(|opt| opt.map(|d| d.whole_nanoseconds() as i64));

        let (data, validity) = collect_with_validity(mapped, 0i64);
        ColumnVector::Duration { data, validity }
    }
}

// --- Non-Option Vectors (100% Validity) ---

impl From<Vec<f64>> for ColumnVector {
    fn from(data: Vec<f64>) -> Self {
        ColumnVector::Float64 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<f32>> for ColumnVector {
    fn from(data: Vec<f32>) -> Self {
        ColumnVector::Float32 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<i64>> for ColumnVector {
    fn from(data: Vec<i64>) -> Self {
        ColumnVector::Int64 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<i32>> for ColumnVector {
    fn from(data: Vec<i32>) -> Self {
        ColumnVector::Int32 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<i16>> for ColumnVector {
    fn from(data: Vec<i16>) -> Self {
        ColumnVector::Int16 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<i8>> for ColumnVector {
    fn from(data: Vec<i8>) -> Self {
        ColumnVector::Int8 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<u64>> for ColumnVector {
    fn from(data: Vec<u64>) -> Self {
        ColumnVector::UInt64 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<u32>> for ColumnVector {
    fn from(data: Vec<u32>) -> Self {
        ColumnVector::UInt32 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<bool>> for ColumnVector {
    fn from(data: Vec<bool>) -> Self {
        ColumnVector::Boolean {
            data,
            validity: None,
        }
    }
}

impl From<Vec<String>> for ColumnVector {
    fn from(data: Vec<String>) -> Self {
        ColumnVector::String {
            data,
            validity: None,
        }
    }
}

impl From<Vec<&str>> for ColumnVector {
    fn from(v: Vec<&str>) -> Self {
        // Using String::from is the idiomatic way to transfer ownership of a slice.
        // Rust's collect() is efficient here as it knows the size of the iterator.
        let data: Vec<String> = v.into_iter().map(String::from).collect();

        ColumnVector::String {
            data,
            validity: None,
        }
    }
}

// --- Non-Nullable Temporal Variant Implementations ---

impl From<Vec<Date>> for ColumnVector {
    /// Maps Date objects to i32 days relative to Unix Epoch (1970-01-01).
    /// Standard epoch-day representation (Industrial standard).
    fn from(v: Vec<Date>) -> Self {
        let unix_epoch = Date::from_calendar_date(1970, time::Month::January, 1).unwrap();
        let data: Vec<i32> = v
            .into_iter()
            .map(|d| (d - unix_epoch).whole_days() as i32)
            .collect();

        ColumnVector::Date {
            data,
            validity: None,
        }
    }
}

impl From<Vec<OffsetDateTime>> for ColumnVector {
    /// Projects OffsetDateTime objects into i64 nanoseconds.
    /// Retains the maximum precision provided by the input objects.
    fn from(v: Vec<OffsetDateTime>) -> Self {
        let data: Vec<i64> = v
            .into_iter()
            .map(|dt| dt.unix_timestamp_nanos() as i64)
            .collect();

        ColumnVector::Datetime {
            data,
            validity: None,
            timezone: None,
        }
    }
}

impl From<Vec<Time>> for ColumnVector {
    /// Maps Time objects to i64 nanoseconds elapsed since 00:00:00.
    /// Uses MIDNIGHT as the reference to capture full nanosecond fidelity.
    fn from(v: Vec<Time>) -> Self {
        let data: Vec<i64> = v
            .into_iter()
            .map(|t| (t - Time::MIDNIGHT).whole_nanoseconds() as i64)
            .collect();

        ColumnVector::Time {
            data,
            validity: None,
        }
    }
}

impl From<Vec<Duration>> for ColumnVector {
    /// Maps Duration objects to i64 nanoseconds.
    /// Preserves raw duration magnitude in high precision.
    fn from(v: Vec<Duration>) -> Self {
        let data: Vec<i64> = v
            .into_iter()
            .map(|d| d.whole_nanoseconds() as i64)
            .collect();

        ColumnVector::Duration {
            data,
            validity: None,
        }
    }
}

/// Helper function to create a validity bitmask from an iterator of Options.
/// Returns (DataVec, ValidityMask).
fn collect_with_validity<T, I>(iter: I, default: T) -> (Vec<T>, Option<Vec<u8>>)
where
    I: IntoIterator<Item = Option<T>>,
    T: Clone,
{
    let iter = iter.into_iter();
    let (lower, _) = iter.size_hint();

    // Optimization: Pre-allocate memory based on size_hint
    let mut data = Vec::with_capacity(lower);
    let mut validity = Vec::with_capacity(lower.div_ceil(8));
    let mut has_nulls = false;

    let mut current_byte = 0u8;
    let mut bit_count = 0;

    for opt in iter {
        match opt {
            Some(v) => {
                data.push(v);
                // Set the corresponding bit to 1 (Valid) using LSB order
                current_byte |= 1 << (bit_count % 8);
            }
            None => {
                data.push(default.clone());
                has_nulls = true;
                // Bit remains 0 (Null)
            }
        }

        bit_count += 1;
        // When a byte is full (8 bits), push it to the vector
        if bit_count % 8 == 0 {
            validity.push(current_byte);
            current_byte = 0;
        }
    }

    // Handle the last partial byte if it exists
    if bit_count % 8 != 0 {
        validity.push(current_byte);
    }

    // MEMORY OPTIMIZATION: If no nulls were encountered, we drop the bitmask
    // to save memory and CPU cycles in downstream SIMD operations.
    let validity_mask = if has_nulls && !data.is_empty() {
        Some(validity)
    } else {
        None
    };

    (data, validity_mask)
}

/// A convenience trait to improve the ergonomics of manual data construction.
///
/// This trait provides the `.into_column()` method for any type that can be
/// converted into a `ColumnVector`. It makes batch ingestion (like using
/// `to_dataset`) more readable by being explicit about the target type.
pub trait IntoColumn {
    fn into_column(self) -> ColumnVector;
}

impl<T> IntoColumn for T
where
    T: Into<ColumnVector>,
{
    #[inline]
    fn into_column(self) -> ColumnVector {
        self.into()
    }
}

/// Universal bridge for fixed-size arrays: [T; N] -> ColumnVector.
impl<Item, const N: usize> From<[Item; N]> for ColumnVector
where
    Vec<Item>: Into<ColumnVector>,
    // Item doesn't need Clone here if we consume the array
{
    fn from(arr: [Item; N]) -> Self {
        // Vec::from(arr) is efficient and consumes the array without unnecessary cloning
        Vec::from(arr).into()
    }
}

/// Bridge for array references: &[T; N] -> ColumnVector.
impl<Item, const N: usize> From<&[Item; N]> for ColumnVector
where
    Vec<Item>: Into<ColumnVector>,
    Item: Clone,
{
    fn from(arr: &[Item; N]) -> Self {
        // Here we must clone, so to_vec() is appropriate
        arr.to_vec().into()
    }
}

/// Bridge for slices: &[Item] -> ColumnVector.
impl<Item> From<&[Item]> for ColumnVector
where
    Vec<Item>: Into<ColumnVector>,
    Item: Clone,
{
    fn from(slice: &[Item]) -> Self {
        slice.to_vec().into()
    }
}

/// Bridge for Vec references: &Vec<Item> -> ColumnVector.
impl<Item> From<&Vec<Item>> for ColumnVector
where
    Vec<Item>: Into<ColumnVector>,
    Item: Clone,
{
    fn from(v: &Vec<Item>) -> Self {
        // Directly clone the Vec instead of going through a slice
        v.clone().into()
    }
}

/// Get data from a column vector.
/// Internal trait to bridge ColumnVector and concrete Rust physical types.
pub trait FromColumnVector: Sized {
    /// Attempts to retrieve a reference to the underlying data slice.
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]>;
}

// --- Floating Point ---

impl FromColumnVector for f64 {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        if let ColumnVector::Float64 { data, .. } = col {
            Some(data)
        } else {
            None
        }
    }
}

impl FromColumnVector for f32 {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        if let ColumnVector::Float32 { data, .. } = col {
            Some(data)
        } else {
            None
        }
    }
}

// --- Integers (The Specialized Trio) ---

impl FromColumnVector for i64 {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        match col {
            ColumnVector::Int64 { data, .. } => Some(data),
            ColumnVector::Datetime { data, .. } => Some(data),
            ColumnVector::Time { data, .. } => Some(data),
            ColumnVector::Duration { data, .. } => Some(data),
            _ => None,
        }
    }
}

impl FromColumnVector for i32 {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        match col {
            ColumnVector::Int32 { data, .. } => Some(data),
            ColumnVector::Date { data, .. } => Some(data),
            _ => None,
        }
    }
}

impl FromColumnVector for u32 {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        match col {
            ColumnVector::UInt32 { data, .. } => Some(data),
            ColumnVector::Categorical { keys, .. } => Some(keys),
            _ => None,
        }
    }
}

// --- Remaining Fixed-size Integers ---

impl FromColumnVector for i16 {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        if let ColumnVector::Int16 { data, .. } = col {
            Some(data)
        } else {
            None
        }
    }
}

impl FromColumnVector for i8 {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        if let ColumnVector::Int8 { data, .. } = col {
            Some(data)
        } else {
            None
        }
    }
}

impl FromColumnVector for u64 {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        if let ColumnVector::UInt64 { data, .. } = col {
            Some(data)
        } else {
            None
        }
    }
}

// --- Boolean & String ---

impl FromColumnVector for bool {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        if let ColumnVector::Boolean { data, .. } = col {
            Some(data)
        } else {
            None
        }
    }
}

impl FromColumnVector for String {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
        if let ColumnVector::String { data, .. } = col {
            Some(data)
        } else {
            None
        }
    }
}

/// Represents the result of a grouping operation, preserving the order of appearance.
pub struct GroupedIndices {
    /// - `Option<String>`: The group label (formatted string representation).
    /// - `Vec<usize>`: Original row indices.
    pub groups: Vec<(Option<String>, Vec<usize>)>,
}

#[derive(Hash, PartialEq, Eq)]
enum InternalKey {
    /// Handles i64 (Time/Datetime/Duration), i32 (Date), and raw bits of floats.
    UInt(u64),
    /// Fallback for true string columns.
    Str(String),
    /// Represents Null values across all types.
    Null,
}

#[derive(Clone, Default)]
pub struct Dataset {
    /// Maps column names to their index in the `columns` vector.
    pub(crate) schema: AHashMap<String, usize>,
    /// Arc-wrapped columns for zero-copy sharing and threading safety.
    pub(crate) columns: Vec<Arc<ColumnVector>>,
    /// Total row count. Must be consistent across all columns.
    pub(crate) row_count: usize,
}

impl Dataset {
    pub fn new() -> Self {
        Self::default()
    }

    /// Internal helper to validate row length consistency.
    ///
    /// NOTE: The first non-empty column added defines the mandatory length
    /// for all subsequent columns in this Dataset.
    fn validate_len(&mut self, name: &str, incoming_len: usize) -> Result<(), ChartonError> {
        if self.columns.is_empty() {
            self.row_count = incoming_len;
            Ok(())
        } else if incoming_len != self.row_count {
            // Error contains the column name to help users identify the source of mismatch.
            Err(ChartonError::Data(format!(
                "Inconsistent column length in '{}': expected {} rows, found {}",
                name, self.row_count, incoming_len
            )))
        } else {
            Ok(())
        }
    }

    /// Returns true if the dataset has no rows.
    pub const fn is_empty(&self) -> bool {
        self.row_count == 0
    }

    /// Adds a new column to the dataset (Imperative Style).
    /// If a column with the same name already exists, it is overwritten with the new data.
    ///
    /// ### When to use:
    /// - Inside loops or conditional logic where columns are added dynamically.
    /// - When you only have a mutable reference (&mut self) to the dataset.
    pub fn add_column<S, V>(&mut self, name: S, data: V) -> Result<(), ChartonError>
    where
        S: Into<String>,
        V: Into<ColumnVector>,
    {
        let name_str = name.into();
        let vec: ColumnVector = data.into();

        // 1. Ensure the new column matches the dataset's row count (if not the first column)
        self.validate_len(&name_str, vec.len())?;

        // 2. Check if the column already exists in the schema
        if let Some(&index) = self.schema.get(&name_str) {
            // 3a. Overwrite existing column data at the stored index
            self.columns[index] = Arc::new(vec);
        } else {
            // 3b. Add as a brand new column
            let index = self.columns.len();
            self.columns.push(Arc::new(vec));
            self.schema.insert(name_str, index);
        }

        Ok(())
    }

    /// Adds a column and returns the Dataset (Fluent/Builder Style).
    ///
    /// ### When to use:
    /// - During initial setup for a clean, readable, and immutable declaration.
    /// - When passing a newly created dataset directly into other functions.
    pub fn with_column<S, V>(mut self, name: S, data: V) -> Result<Self, ChartonError>
    where
        S: Into<String>,
        V: Into<ColumnVector>,
    {
        // Reuse add_column to avoid logic duplication
        self.add_column(name, data)?;
        Ok(self)
    }

    /// Returns the number of rows in the dataset.
    pub const fn height(&self) -> usize {
        self.row_count
    }

    /// Returns the number of columns in the dataset.
    pub const fn width(&self) -> usize {
        self.columns.len()
    }

    /// Returns column names in their order of insertion.
    /// Optimized to avoid unnecessary sorting.
    pub fn get_column_names(&self) -> Vec<String> {
        // Since we know the number of columns, we pre-allocate.
        let mut names = vec![String::new(); self.columns.len()];

        for (name, &idx) in self.schema.iter() {
            // Safety: idx is guaranteed to be within bounds by add_column logic.
            if let Some(slot) = names.get_mut(idx) {
                *slot = name.clone();
            }
        }
        names
    }

    /// Creates a lightweight subset of the dataset containing only the requested columns.
    ///
    /// This is useful for quick inspection or debugging, for example:
    /// `println!("{:?}", ds.select(&["a", "b"])?);`
    pub fn select<S>(&self, names: &[S]) -> Result<Self, ChartonError>
    where
        S: AsRef<str>,
    {
        let mut subset = Self::new();
        subset.row_count = self.row_count;

        for name in names {
            let key = name.as_ref();
            let &index = self
                .schema
                .get(key)
                .ok_or_else(|| ChartonError::Data(format!("Column '{}' not found", key)))?;

            subset.columns.push(self.columns[index].clone());
            subset
                .schema
                .insert(key.to_string(), subset.columns.len() - 1);
        }

        Ok(subset)
    }

    /// Returns a reference to the [ColumnVector] wrapper.
    /// Use this when you need to inspect metadata like TimeUnit or Validity masks.
    pub fn column(&self, name: &str) -> Result<&ColumnVector, ChartonError> {
        let index = self
            .schema
            .get(name)
            .ok_or_else(|| ChartonError::Data(format!("Column '{}' not found", name)))?;
        Ok(&self.columns[*index])
    }

    /// High-performance: Returns a reference to the underlying physical data slice.
    ///
    /// ### Warning:
    /// For temporal types (Datetime, Time, Duration), this returns the raw i64 values.
    /// Use `dataset.column(name)` to check units (e.g., Nanoseconds) if needed.
    pub fn get_column<T: FromColumnVector>(&self, name: &str) -> Result<&[T], ChartonError> {
        let col = self.column(name)?; // Reuse the column() helper

        T::try_from_col(col).ok_or_else(|| {
            ChartonError::Data(format!(
                "Type mismatch: Column '{}' cannot be accessed as the requested type",
                name
            ))
        })
    }

    /// Unified high-level entry point to retrieve a dynamic scalar cell.
    pub fn get(&self, name: &str, row: usize) -> AnyValue<'_> {
        self.column(name)
            .map(|col| col.get(row))
            .unwrap_or(AnyValue::Null)
    }

    /// [WASM PREPARATION]: Zero-Allocation Data Streaming.
    /// Modifies a Float64 column in place, bypassing the standard Arc clone
    /// penalty by utilizing `Arc::get_mut` during single-threaded WASM loops.
    pub fn update_column_f64(&mut self, name: &str, new_data: &[f64]) -> Result<(), ChartonError> {
        self.validate_len(name, new_data.len())?;

        if let Some(&index) = self.schema.get(name) {
            // Attempt to acquire mutable reference (succeeds if Arc strong count is 1)
            if let Some(col) = Arc::get_mut(&mut self.columns[index]) {
                col.update_f64_data(new_data)?;
            } else {
                // Fallback: If the column is deeply shared (e.g., heavily borrowed in multi-threading),
                // we gracefully degrade to a fresh allocation.
                self.columns[index] = Arc::new(ColumnVector::Float64 {
                    data: new_data.to_vec(),
                    validity: None,
                });
            }
            Ok(())
        } else {
            Err(ChartonError::Data(format!(
                "Column '{}' not found for update",
                name
            )))
        }
    }

    /// Checks if a value at a specific row is null.
    ///
    /// This is a "safe" check that handles missing columns gracefully.
    /// It returns `true` if the column does not exist, if the row is masked
    /// as null, or if the numerical value is NaN.
    pub fn is_null(&self, name: &str, row: usize) -> bool {
        self.column(name)
            .map(|col| col.is_null(row))
            .unwrap_or(true)
    }

    /// Generates a combined bitmask for multiple columns.
    ///
    /// This is a high-performance "AND" operation across multiple validity maps.
    /// It ensures that a row is only marked as valid (1) if it is valid in ALL
    /// specified columns, including implicit NaN checks for floating-point data.
    pub fn get_combined_mask(&self, column_names: &[&str]) -> Result<Vec<u8>, ChartonError> {
        if self.row_count == 0 {
            return Ok(Vec::new());
        }

        let byte_count = self.row_count.div_ceil(8);
        let mut final_mask = vec![0xFFu8; byte_count];

        for &name in column_names {
            let col = self.column(name)?;

            // 1. Parallel Bitwise AND: Merge the existing validity masks.
            if let Some(v) = col.get_validity_mask() {
                // zip() combined with iter_mut() is highly optimized by LLVM.
                for (m, b) in final_mask.iter_mut().zip(v.iter()) {
                    *m &= *b;
                }
            }

            // 2. Implicit Nulls: Floating-point NaNs.
            // We scan data in chunks of 8 to minimize mask memory writes.
            match col {
                ColumnVector::Float64 { data, .. } => {
                    self.apply_nan_mask_f64(data, &mut final_mask);
                }
                ColumnVector::Float32 { data, .. } => {
                    self.apply_nan_mask_f32(data, &mut final_mask);
                }
                _ => {}
            }
        }

        // 3. Tail Cleanup: Zero out unused bits in the final byte.
        let remainder = self.row_count & 7; // Equal to % 8
        if remainder != 0
            && let Some(last_byte) = final_mask.last_mut()
        {
            let padding_mask = (1 << remainder) - 1;
            *last_byte &= padding_mask;
        }

        Ok(final_mask)
    }

    /// Internal helper to scan f64 slices for NaNs and update the bitmask.
    fn apply_nan_mask_f64(&self, data: &[f64], mask: &mut [u8]) {
        // Processing in chunks of 8 elements (matching 1 byte in the mask)
        data.chunks(8).enumerate().for_each(|(i, chunk)| {
            let mut nan_byte = 0xFFu8;
            for (bit_idx, val) in chunk.iter().enumerate() {
                if val.is_nan() {
                    nan_byte &= !(1 << bit_idx);
                }
            }
            // Only one memory write per 8 floats
            mask[i] &= nan_byte;
        });
    }

    /// Internal helper to scan f32 slices for NaNs and update the bitmask.
    fn apply_nan_mask_f32(&self, data: &[f32], mask: &mut [u8]) {
        data.chunks(8).enumerate().for_each(|(i, chunk)| {
            let mut nan_byte = 0xFFu8;
            for (bit_idx, val) in chunk.iter().enumerate() {
                if val.is_nan() {
                    nan_byte &= !(1 << bit_idx);
                }
            }
            mask[i] &= nan_byte;
        });
    }

    /// Performs a high-performance row selection and reordering across the entire dataset.
    /// Preserves the original column ordering and schema integrity.
    pub fn take_rows(&self, indices: &[usize]) -> Result<Self, ChartonError> {
        let h = self.height();
        let new_len = indices.len();

        // 1. Pre-validation: Ensure all indices are within bounds.
        // We do this upfront to guarantee an Atomic-like failure (all or nothing).
        for &idx in indices {
            if idx >= h {
                return Err(ChartonError::Data(format!(
                    "Index {} is out of bounds for Dataset with height {}",
                    idx, h
                )));
            }
        }

        // 2. Prepare the new containers with pre-allocated capacity.
        let mut new_columns = Vec::with_capacity(self.columns.len());

        // 3. Iterate by index to preserve the original column order.
        // We rebuild the names list to reconstruct the schema efficiently.
        let names = self.get_column_names();

        for name in names {
            let col_idx = self.schema[&name];
            let old_col = &self.columns[col_idx];

            // Perform the physical 'take' operation
            let new_col = old_col.take(indices);
            new_columns.push(Arc::new(new_col));
        }

        // 4. Construct the new Dataset directly to bypass redundant validations
        let mut new_schema = AHashMap::with_capacity(self.columns.len());
        for (i, name) in self.get_column_names().into_iter().enumerate() {
            new_schema.insert(name, i);
        }

        Ok(Self {
            schema: new_schema,
            columns: new_columns,
            row_count: new_len,
        })
    }

    /// Partitions the dataset using aHash and Rayon (if enabled) for maximum throughput,
    /// while preserving the order of groups based on their first appearance.
    pub fn group_by(&self, col_name: Option<&str>) -> GroupedIndices {
        // 1. Resolve the grouping column.
        let col_vector = col_name.and_then(|name| self.column(name).ok());

        // 2. Handle the "No Grouping" case.
        let vector = match col_vector {
            Some(v) => v,
            None => {
                return GroupedIndices {
                    groups: vec![(None, (0..self.row_count).collect())],
                };
            }
        };

        // 3. Dispatch based on the "parallel" feature.
        #[cfg(feature = "parallel")]
        {
            self.group_by_parallel(vector)
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.group_by_serial(vector)
        }
    }

    #[cfg(feature = "parallel")]
    fn group_by_parallel(&self, vector: &ColumnVector) -> GroupedIndices {
        use rayon::prelude::*;

        // Map<InternalKey, (FirstSeenIndex, Vec<RowIndices>)>
        let groups_map = (0..self.row_count)
            .into_par_iter()
            .fold(
                || AHashMap::<InternalKey, (usize, Vec<usize>)>::with_capacity(128),
                |mut local_map, i| {
                    let key = self.get_internal_key(vector, i);
                    local_map
                        .entry(key)
                        .and_modify(|(_, indices)| indices.push(i))
                        .or_insert_with(|| (i, vec![i]));
                    local_map
                },
            )
            .reduce(AHashMap::default, |mut map1, mut map2| {
                // Optimization: Always merge the smaller map into the larger one.
                if map1.len() < map2.len() {
                    std::mem::swap(&mut map1, &mut map2);
                }
                for (key, (first_idx2, mut indices2)) in map2.drain() {
                    map1.entry(key)
                        .and_modify(|(first_idx1, indices1)| {
                            if first_idx2 < *first_idx1 {
                                *first_idx1 = first_idx2;
                            }
                            indices1.append(&mut indices2);
                        })
                        .or_insert((first_idx2, indices2));
                }
                map1
            });

        self.finalize_groups(groups_map, vector)
    }

    #[cfg(not(feature = "parallel"))]
    fn group_by_serial(&self, vector: &ColumnVector) -> GroupedIndices {
        let mut groups_map = AHashMap::<InternalKey, (usize, Vec<usize>)>::with_capacity(128);

        for i in 0..self.row_count {
            let key = self.get_internal_key(vector, i);
            groups_map
                .entry(key)
                .and_modify(|(_, indices)| indices.push(i))
                .or_insert_with(|| (i, vec![i]));
        }

        self.finalize_groups(groups_map, vector)
    }

    /// Helper to extract an InternalKey without allocating a String.
    fn get_internal_key(&self, vector: &ColumnVector, i: usize) -> InternalKey {
        // High-performance check for both explicit Nulls and implicit NaNs
        if vector.is_null(i) {
            return InternalKey::Null;
        }

        match vector {
            // Categorical uses the physical u32 key - very fast hashing.
            ColumnVector::Categorical { keys, .. } => InternalKey::UInt(keys[i] as u64),

            ColumnVector::Float64 { data, .. } => InternalKey::UInt(data[i].to_bits()),
            ColumnVector::Float32 { data, .. } => InternalKey::UInt(data[i].to_bits() as u64),
            ColumnVector::Int64 { data, .. } => InternalKey::UInt(data[i] as u64),
            ColumnVector::Int32 { data, .. } => InternalKey::UInt(data[i] as u64),
            ColumnVector::UInt32 { data, .. } => InternalKey::UInt(data[i] as u64),
            ColumnVector::UInt64 { data, .. } => InternalKey::UInt(data[i]),
            ColumnVector::Datetime { data, .. } => InternalKey::UInt(data[i] as u64),

            // Strings require cloning, which is the slowest path.
            ColumnVector::String { data, .. } => InternalKey::Str(data[i].clone()),
            _ => InternalKey::Null,
        }
    }

    /// Finalizes groups by converting InternalKeys back to Option<String>
    /// and sorting by the first-seen index to preserve input order.
    fn finalize_groups(
        &self,
        groups_map: AHashMap<InternalKey, (usize, Vec<usize>)>,
        vector: &ColumnVector,
    ) -> GroupedIndices {
        // 1. Convert HashMap to Vec for sorting.
        let mut sorted_groups: Vec<(InternalKey, (usize, Vec<usize>))> =
            groups_map.into_iter().collect();

        // 2. Sort groups based on their first appearance to maintain stability.
        sorted_groups.sort_by_key(|(_, (first_idx, _))| *first_idx);

        // 3. Transform InternalKey back to human-readable Option<String>.
        let groups = sorted_groups
            .into_iter()
            .map(|(key, (_, mut indices))| {
                let label = match key {
                    InternalKey::UInt(val) => match vector {
                        // For Categorical, we pull the string from the dictionary.
                        ColumnVector::Categorical { values, .. } => {
                            values.get(val as usize).cloned()
                        }
                        // For other numeric types, we format as string.
                        _ => Some(val.to_string()),
                    },
                    InternalKey::Str(s) => Some(s),
                    InternalKey::Null => None,
                };

                // Sort row indices for better memory locality during subsequent "take" operations.
                indices.sort_unstable();
                (label, indices)
            })
            .collect();

        GroupedIndices { groups }
    }

    /// Constructs a Dataset from a slice of Apache Arrow RecordBatches.
    ///
    /// This method is designed for general-purpose Arrow compatibility (e.g., data
    /// from Parquet files, databases, or Arrow Flight). It automatically
    /// concatenates fragmented chunks into unified arrays before conversion.
    ///
    /// # Implementation Note
    /// While optimized with Arrow's bitwise concatenation kernel, this method
    /// may involve significant memory copying for very large datasets. For
    /// Polars-originated data, prefer `from_arrays` via the `load_polars_df!` macro.
    #[cfg(feature = "arrow")]
    pub fn from_record_batches(
        batches: &[arrow::record_batch::RecordBatch],
    ) -> Result<Self, ChartonError> {
        use arrow::array::{Array, Float32Array, Float64Array, Int64Array, StringArray};
        use arrow::datatypes::{DataType, TimeUnit};

        if batches.is_empty() {
            return Ok(Self::new());
        }

        // All batches in a stream must share the same schema.
        let schema = batches[0].schema();
        let mut dataset = Self::new();

        // Process columns one by one to keep memory access patterns predictable.
        for (i, field) in schema.fields().iter().enumerate() {
            // 1. Gather all chunks (RecordBatches) for the current column.
            let column_arrays: Vec<&dyn arrow::array::Array> =
                batches.iter().map(|b| b.column(i).as_ref()).collect();

            // 2. Unify fragmented chunks into a single contiguous Arrow array.
            // This is a physical memory copy operation (Concatenation).
            let merged_array = arrow::compute::concat(&column_arrays)
                .map_err(|e| ChartonError::Data(format!("Arrow concat error: {}", e)))?;

            // 3. Perform type-specific conversion to Charton's internal format.
            let column_vector = ColumnVector::from_arrow(merged_array.as_ref())?;

            dataset.add_column(field.name(), column_vector)?;
        }

        Ok(dataset)
    }

    /// EAGER: Returns a new Dataset containing the first `n` rows.
    /// This creates a shallow copy where ColumnVectors are sliced and re-wrapped in Arc.
    pub fn head(&self, n: usize) -> Self {
        let actual_n = n.min(self.row_count);
        self.slice(0, actual_n)
    }

    /// EAGER: Returns a new Dataset containing the last `n` rows.
    /// Useful for extracting the most recent entries in a dataset.
    pub fn tail(&self, n: usize) -> Self {
        let actual_n = n.min(self.row_count);
        let offset = self.row_count - actual_n;
        self.slice(offset, actual_n)
    }

    /// Creates a new owned Dataset from a sub-range of the current one.
    /// It clones the Schema and creates new Sliced ColumnVectors.
    pub fn slice(&self, offset: usize, len: usize) -> Self {
        if len == 0 {
            return Self::new();
        }

        // Each column is sliced independently. Since we use Arc,
        // we are creating new Arcs pointing to the new sliced vectors.
        let new_columns: Vec<Arc<ColumnVector>> = self
            .columns
            .iter()
            .map(|col| Arc::new(col.slice(offset, len)))
            .collect();

        Self {
            schema: self.schema.clone(), // Shallow clone of the AHashMap
            columns: new_columns,
            row_count: len,
        }
    }

    /// Internal helper to convert a specific cell value into a string for display.
    fn debug_cell(&self, col_name: &str, row: usize) -> String {
        // 1. Unified Null Check (Handles both NaN and Validity Bitmaps)
        if self.is_null(col_name, row) {
            return "null".to_string();
        }

        // 2. Resolve column index safely.
        let idx = match self.schema.get(col_name) {
            Some(&i) => i,
            None => {
                return format!("err: col '{}' not found", col_name);
            }
        };

        // 3. Format based on physical storage type
        match &*self.columns[idx] {
            // --- Boolean ---
            ColumnVector::Boolean { data, .. } => data[row].to_string(),

            // --- Floating Point (Precision clamped to 4 decimal places) ---
            ColumnVector::Float64 { data, .. } => format!("{:.4}", data[row]),
            ColumnVector::Float32 { data, .. } => format!("{:.4}", data[row]),

            // --- Integers (All widths) ---
            ColumnVector::Int8 { data, .. } => data[row].to_string(),
            ColumnVector::Int16 { data, .. } => data[row].to_string(),
            ColumnVector::Int32 { data, .. } => data[row].to_string(),
            ColumnVector::Int64 { data, .. } => data[row].to_string(),
            ColumnVector::UInt32 { data, .. } => data[row].to_string(),
            ColumnVector::UInt64 { data, .. } => data[row].to_string(),

            // --- Categorical & Strings ---
            ColumnVector::Categorical { keys, values, .. } => {
                let key = keys[row] as usize;
                values
                    .get(key)
                    .map(|s| s.as_str())
                    .unwrap_or("err_key")
                    .to_string()
            }
            ColumnVector::String { data, .. } => data[row].to_string(),

            // --- Temporal Types ---

            // Date: i32 days since Epoch
            ColumnVector::Date { data, .. } => {
                use time::{Duration, OffsetDateTime};
                let unix_epoch = OffsetDateTime::UNIX_EPOCH;
                let date = unix_epoch + Duration::days(data[row] as i64);
                date.date().to_string()
            }

            // Datetime: i64 since Epoch with specific precision
            ColumnVector::Datetime { data, .. } => {
                use time::{OffsetDateTime, format_description::well_known::Rfc3339};

                // Convert various units to Nanoseconds using i128 to avoid overflow.
                // i64 seconds * 1e9 safely fits in i128.
                let timestamp_nanos = data[row] as i128;

                OffsetDateTime::from_unix_timestamp_nanos(timestamp_nanos)
                    .ok()
                    .and_then(|dt| dt.format(&Rfc3339).ok())
                    .unwrap_or_else(|| "out_of_range_datetime".to_string())
            }

            // Duration: Physical count of units
            ColumnVector::Duration { data, .. } => data[row].to_string(),

            // Time: i64 nanoseconds since midnight
            ColumnVector::Time { data, .. } => {
                use time::Time;
                // Use rem_euclid to handle potential negative offsets safely
                let nanos_in_day = (data[row] % 86_400_000_000_000).unsigned_abs() as u32;
                Time::from_hms_nano(0, 0, 0, nanos_in_day)
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "err_time".to_string())
            }
        }
    }

    /// Internal rendering engine that formats a specific range of rows as a table.
    /// This implementation includes a Polars-style type row (e.g., (str), (f64))
    /// below each column header for better data inspection.
    fn render_to_format(
        &self,
        f: &mut fmt::Formatter<'_>,
        offset: usize,
        len: usize,
    ) -> fmt::Result {
        // 1. Boundary Check: Ensure we don't scan past the end of the dataset
        let actual_len = len.min(self.row_count.saturating_sub(offset));
        let end_index = offset + actual_len;

        writeln!(
            f,
            "Dataset View: rows {}..{} (Total {} rows)",
            offset, end_index, self.row_count
        )?;

        if actual_len == 0 && self.row_count > 0 && offset >= self.row_count {
            return writeln!(f, "Empty view (offset out of bounds)");
        }

        // 2. Sort column names based on their insertion order (index in schema)
        let mut names: Vec<_> = self.schema.keys().collect();
        names.sort_by_key(|name| self.schema.get(*name).expect("Schema integrity error"));

        if names.is_empty() {
            return Ok(());
        }

        // ======================== Core Fix: Calculate per-column width (Polars-style) ========================
        let mut column_widths = Vec::new();
        for name in &names {
            // Length of the column header name
            let name_len = name.chars().count();

            // Length of the data type label, e.g., (str), (f64)
            let dtype = self
                .column(name)
                .map(|col| col.dtype_name())
                .unwrap_or("unknown");
            let type_label = format!("({})", dtype);
            let type_len = type_label.chars().count();

            // Find the longest string representation in the current data window
            let mut max_data_len = 0;
            for row in offset..end_index {
                let cell = self.debug_cell(name, row);
                let len = cell.chars().count();
                if len > max_data_len {
                    max_data_len = len;
                }
            }

            // Final column width = MAX(header, dtype, data)
            let width = *[name_len, type_len, max_data_len].iter().max().unwrap();
            column_widths.push(width);
        }

        // ======================== Print Header Row ========================
        for (i, (name, &width)) in names.iter().zip(&column_widths).enumerate() {
            if i > 0 {
                write!(f, "| ")?;
            }
            write!(f, "{:<width$}", name, width = width)?;
        }
        writeln!(f)?;

        // ======================== Print Data Type Row ========================
        for (i, (name, &width)) in names.iter().zip(&column_widths).enumerate() {
            if i > 0 {
                write!(f, "| ")?;
            }
            let dtype = self
                .column(name)
                .map(|col| col.dtype_name())
                .unwrap_or("unknown");
            let type_label = format!("({})", dtype);
            write!(f, "{:<width$}", type_label, width = width)?;
        }
        writeln!(f)?;

        // ======================== Print Separator Line ========================
        let total_sep: usize = column_widths.iter().sum::<usize>() + 2 * (names.len() - 1);
        writeln!(f, "{}", "-".repeat(total_sep))?;

        // ======================== Print Data Rows ========================
        for row in offset..end_index {
            for (i, (name, &width)) in names.iter().zip(&column_widths).enumerate() {
                if i > 0 {
                    write!(f, "| ")?;
                }
                let cell = self.debug_cell(name, row);
                write!(f, "{:<width$}", cell, width = width)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }

    /// Returns a lightweight [DatasetView] for a specific range.
    /// Used internally for printing or quick data inspection without allocations.
    pub fn view(&self, offset: usize, len: usize) -> DatasetView<'_> {
        let safe_len = if offset >= self.row_count {
            0
        } else {
            len.min(self.row_count - offset)
        };

        DatasetView {
            ds: self,
            offset,
            len: safe_len,
        }
    }
}

/// A lightweight view of a Dataset, typically created via `head()` or `tail()`.
/// This struct is public so it can be used in type signatures,
/// but its fields remain private to ensure data integrity.
pub struct DatasetView<'a> {
    pub(crate) ds: &'a Dataset,
    pub(crate) offset: usize,
    pub(crate) len: usize,
}

impl<'a> std::fmt::Debug for DatasetView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.ds.render_to_format(f, self.offset, self.len)
    }
}

impl fmt::Debug for Dataset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Just print a 10-row view
        self.view(0, 10).fmt(f)?;

        if self.row_count > 10 {
            writeln!(f, "... and {} more rows", self.row_count - 10)?;
        }
        Ok(())
    }
}

// --- ToDataset Ingestion Trait ---

pub trait ToDataset {
    fn to_dataset(self) -> Result<Dataset, ChartonError>;
}

impl<I, S, V> ToDataset for I
where
    I: IntoIterator<Item = (S, V)>,
    S: Into<String>,
    V: Into<ColumnVector>,
{
    fn to_dataset(self) -> Result<Dataset, ChartonError> {
        let mut ds = Dataset::new();
        for (name, data) in self {
            ds.add_column(name, data)?;
        }
        Ok(ds)
    }
}

/// Identity conversion for an already-constructed Dataset.
impl ToDataset for Dataset {
    #[inline]
    fn to_dataset(self) -> Result<Dataset, ChartonError> {
        Ok(self)
    }
}

/// Identify conversion for a reference of a Dataset.
impl ToDataset for &Dataset {
    #[inline]
    fn to_dataset(self) -> Result<Dataset, ChartonError> {
        // Since it uses Arc internally, this clone only increments the reference count and is extremely fast.
        Ok(self.clone())
    }
}

/// A lightweight accessor to fetch values from a specific row in a Dataset.
///
/// It is designed to be created frequently inside loops, providing a clean
/// interface for closures while maintaining high performance.
#[derive(Copy, Clone)]
pub struct RowAccessor<'a> {
    ds: &'a Dataset,
    current_row: usize,
}

impl<'a> RowAccessor<'a> {
    /// Creates a new RowAccessor for a specific row.
    pub const fn new(ds: &'a Dataset, row: usize) -> Self {
        Self {
            ds,
            current_row: row,
        }
    }

    /// Fetches a numeric value from the specified field.
    /// Returns None if the column doesn't exist or the value is Null.
    #[inline]
    pub fn val(&self, field: &str) -> Option<f64> {
        self.ds.get(field, self.current_row).to_f64()
    }

    /// Fetches a string value from the specified field.
    /// Returns None if the column doesn't exist or the value is Null.
    #[inline]
    pub fn str(&self, field: &str) -> Option<String> {
        self.ds.get(field, self.current_row).to_string()
    }

    /// Returns the current row index.
    pub const fn index(&self) -> usize {
        self.current_row
    }
}

/// Represents the statistical aggregation operations available for data transformation.
///
/// This enum defines how multiple data points are collapsed into a single value
/// during the transformation phase. It is used both in simple aggregations
/// and complex window functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AggregateOp {
    /// Total sum of all valid (non-null) values in the group.
    #[default]
    Sum,
    /// Arithmetic mean (average). Result is NaN if all values are null.
    Mean,
    /// The middle value. Requires a partial sort of the group data.
    Median,
    /// The smallest value in the group.
    Min,
    /// The largest value in the group.
    Max,
    /// The total count of records (including or excluding nulls, based on implementation).
    Count,
}

impl From<&str> for AggregateOp {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mean" | "avg" => Self::Mean,
            "sum" => Self::Sum,
            "min" => Self::Min,
            "max" => Self::Max,
            "count" | "n" => Self::Count,
            "median" => Self::Median,
            _ => Self::Sum,
        }
    }
}

impl AggregateOp {
    /// Native aggregation logic: Extracting and aggregating data from columns based on indices.
    ///
    /// This method performs statistical calculations directly on the provided
    /// ColumnVector using only the specified row indices.
    pub fn aggregate_by_index(&self, col: &ColumnVector, indices: &[usize]) -> f64 {
        if indices.is_empty() {
            return f64::NAN;
        }

        match self {
            AggregateOp::Count => indices.len() as f64,

            AggregateOp::Sum => {
                let mut sum = 0.0;
                let mut has_valid = false;
                for &i in indices {
                    if let Some(v) = col.get(i).to_f64() {
                        sum += v;
                        has_valid = true;
                    }
                }
                if has_valid { sum } else { f64::NAN }
            }

            AggregateOp::Mean => {
                let mut sum = 0.0;
                let mut count = 0;
                for &i in indices {
                    if let Some(v) = col.get(i).to_f64() {
                        sum += v;
                        count += 1;
                    }
                }
                if count > 0 {
                    sum / count as f64
                } else {
                    f64::NAN
                }
            }

            AggregateOp::Min => indices
                .iter()
                .filter_map(|&i| col.get(i).to_f64())
                .fold(None, |acc: Option<f64>, v| {
                    Some(acc.map_or(v, |m| m.min(v)))
                })
                .unwrap_or(f64::NAN),

            AggregateOp::Max => indices
                .iter()
                .filter_map(|&i| col.get(i).to_f64())
                .fold(None, |acc: Option<f64>, v| {
                    Some(acc.map_or(v, |m| m.max(v)))
                })
                .unwrap_or(f64::NAN),

            AggregateOp::Median => {
                let vals = self.extract_and_sort(col, indices);
                get_quantile(&vals, 0.5)
            }
        }
    }

    fn extract_and_sort(&self, col: &ColumnVector, indices: &[usize]) -> Vec<f64> {
        // Filter out NaNs and Nulls to ensure sort stability
        let mut vals: Vec<f64> = indices
            .iter()
            .filter_map(|&i| col.get(i).to_f64())
            .filter(|v| !v.is_nan())
            .collect();

        // Use total_cmp if available (Rust 1.62+) for a consistent ordering of floats
        vals.sort_unstable_by(|a, b| a.total_cmp(b));
        vals
    }
}

/// Native aggregation logic: Linear interpolation quantile calculation.
pub fn get_quantile(sorted_data: &[f64], q: f64) -> f64 {
    let len = sorted_data.len();
    if len == 0 {
        return f64::NAN;
    }
    let pos = q * (len - 1) as f64;
    let base = pos.floor() as usize;
    let fract = pos - base as f64;

    if base + 1 < len {
        sorted_data[base] + fract * (sorted_data[base + 1] - sorted_data[base])
    } else {
        sorted_data[base]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_dataset_construction_methods() {
        use time::macros::datetime;

        // --- Method 1: Manual Fluent Construction ---
        // Ideal for scenarios where columns are added dynamically during data processing logic.
        let mut ds_manual = Dataset::new();

        // Ingesting raw primitives (assuming they implement IntoColumnVector)
        ds_manual.add_column("id", vec![1i64, 2, 3]).unwrap();

        // Ingesting data with optional values (None will be tracked in the validity bitmap)
        ds_manual
            .add_column("value", vec![Some(10.1), None, Some(30.3)])
            .unwrap();

        assert_eq!(ds_manual.row_count, 3);
        assert!(ds_manual.is_null("value", 1)); // Row 1 should be identified as Null

        // --- Method 2: Automatic Conversion from Tuple Vectors ---
        // This is the most idiomatic way to perform bulk ingestion from key-value pairs.
        let raw_data = vec![
            (
                "name",
                vec![Some("A".to_string()), Some("B".to_string())].into_column(),
            ),
            ("score", vec![100i64, 200i64].into_column()),
        ];

        // Using the ToDataset trait to convert the collection into a structured Dataset
        let ds_from_tuples = raw_data
            .to_dataset()
            .expect("Should convert from tuples successfully");

        assert_eq!(ds_from_tuples.row_count, 2);
        assert_eq!(ds_from_tuples.get("name", 0).to_string().unwrap(), "A");

        // --- Method 3: Complex Mixed-Type Construction ---
        // Verifies that diverse types (DateTime, f32, Strings) coexist within the same Dataset
        // via a unified interface.
        let complex_data = vec![
            (
                "timestamp",
                vec![
                    datetime!(2026-03-30 00:00 UTC),
                    datetime!(2026-03-31 00:00 UTC),
                ]
                .into_column(),
            ),
            ("f32_val", vec![1.1f32, 2.2f32].into_column()),
            ("tags", vec![Some("heavy".to_string()), None].into_column()),
        ];

        let ds_complex = complex_data
            .to_dataset()
            .expect("Should handle heterogeneous types");

        assert_eq!(ds_complex.row_count, 2);
        // Ensure the timestamp is correctly stored and recognized as non-null
        assert!(!ds_complex.is_null("timestamp", 0));
        // Ensure the string 'None' was correctly mapped to the validity bitmap
        assert!(ds_complex.is_null("tags", 1));

        // Print output to verify the Debug implementation with mixed types
        println!("\n--- Construction Method 3 Output ---");
        println!("{:?}", ds_complex);

        // --- Method 4: Pure Functional / Fluent Construction ---
        // Best for static configurations or building datasets without 'mut' variables.
        // It demonstrates how ownership moves through each 'with_column' call.
        let ds_fluent = Dataset::new()
            .with_column("x", vec![10.0, 20.0, 30.0])
            .unwrap()
            .with_column("y", vec![Some(100i64), None, Some(300i64)])
            .unwrap()
            .with_column("category", vec!["A", "B", "C"])
            .unwrap();

        assert_eq!(ds_fluent.row_count, 3);
        assert_eq!(ds_fluent.width(), 3);

        // Verify that even without 'mut', the data is correctly ingested
        assert!(ds_fluent.is_null("y", 1)); // The 'None' value
        assert!(!ds_fluent.is_null("x", 1)); // The float value (20.0) is valid

        println!("\n--- Construction Method 4 (Fluent) Output ---");
        println!("{:?}", ds_fluent);
    }

    #[test]
    fn test_get_column_and_nan_handling() {
        let mut ds = Dataset::new();
        // Ingest data with a NaN value
        let prices = vec![10.5, f64::NAN, 30.2];
        ds.add_column("price", prices).unwrap();

        // Successfully retrieve as f64 slice
        let col = ds.get_column::<f64>("price").expect("Column should exist");
        assert_eq!(col.len(), 3);
        assert_eq!(col[0], 10.5);
        assert!(col[1].is_nan()); // Verify NaN is preserved

        // Verify type safety: requesting as i64 should fail
        let wrong_type = ds.get_column::<i64>("price");
        assert!(wrong_type.is_err());
    }

    #[test]
    fn test_get_value_with_bitmaps() {
        let mut ds = Dataset::new();
        // row 0: Some, row 1: None, row 2: Some
        let ids = vec![Some(100), None, Some(300)];
        ds.add_column("id", ids).unwrap();

        // Check row 0 (Valid)
        assert_eq!(ds.get("id", 0).to_f64().unwrap(), 100.0);
        assert!(!ds.is_null("id", 0));

        // Check row 1 (Null)
        // Note: get_value still returns a reference to the underlying data (likely 0),
        // so is_null is the authoritative way to check validity.
        assert!(ds.is_null("id", 1));

        // Check row 2 (Valid)
        assert_eq!(ds.get("id", 2).to_f64().unwrap(), 300.0);

        // Check out-of-bounds column
        assert!(ds.is_null("non_existent", 0));
    }

    #[test]
    fn test_dataset_display_and_truncation() {
        let mut ds = Dataset::new();

        // Add various types including long strings and dates
        ds.add_column("id", vec![Some(1), Some(2)]).unwrap();
        ds.add_column("city", vec![Some("San Francisco"), None])
            .unwrap();
        ds.add_column("value", vec![1.234567, 8.9]).unwrap();

        // The output should show aligned columns, 'null' for None,
        // and truncated string for "San Francisco" -> "San Fra..."
        println!("\n--- Dataset Debug Output ---");
        println!("{:?}", ds);
        println!("----------------------------");

        assert_eq!(ds.row_count, 2);
    }

    /// This module only exists and compiles when the "arrow" feature is active.
    #[cfg(feature = "arrow")]
    mod arrow_tests {
        use super::*;
        use arrow::array::{Float64Array, Int64Array, StringArray, TimestampMillisecondArray};

        #[test]
        fn test_arrow_ingestion() {
            // 1. Float64 with Nulls (Target: GPU/Canvas optimization)
            let f64_array = Float64Array::from(vec![Some(1.1), None, Some(3.3)]);
            let col_f64 = ColumnVector::from_arrow(&f64_array).expect("F64 ingestion failed");

            if let ColumnVector::F64 { data } = col_f64 {
                assert_eq!(data[0], 1.1);
                assert!(data[1].is_nan()); // NaN is essential for canvas drawing skips
                assert_eq!(data[2], 3.3);
            }

            // 2. Int64 with Nulls (Bitmask verification)
            let i64_array = Int64Array::from(vec![Some(10), None, Some(30)]);
            let col_i64 = ColumnVector::from_arrow(&i64_array).expect("I64 ingestion failed");

            if let ColumnVector::I64 { data, validity } = col_i64 {
                assert_eq!(data, vec![10, 0, 30]); // Default 0 for nulls
                let mask = validity.expect("Validity mask should exist");
                // LSB-first: bit 0 = index 0 (Some), bit 1 = index 1 (None), bit 2 = index 2 (Some)
                // 0b101 is correct for [Valid, Invalid, Valid]
                assert_eq!(mask[0] & 0b111, 0b101);
            }

            // 3. StringArray
            let str_array = StringArray::from(vec![Some("Charton"), None, Some("Rust")]);
            let col_str = ColumnVector::from_arrow(&str_array).expect("String ingestion failed");

            if let ColumnVector::String { data, validity } = col_str {
                assert_eq!(data[0], "Charton");
                assert_eq!(data[1], ""); // Standard empty filler
                assert_eq!(data[2], "Rust");
                assert!(validity.is_some());
            }

            // 4. Timestamp (Millisecond)
            // 1711872000000 ms = 2024-03-31T08:00:00Z
            let ts_array = TimestampMillisecondArray::from(vec![Some(1711872000000), None]);
            let col_ts = ColumnVector::from_arrow(&ts_array).expect("Timestamp ingestion failed");

            if let ColumnVector::DateTime { data, validity, .. } = col_ts {
                // Verify year extraction
                assert_eq!(data[0].year(), 2024);
                assert_eq!(data[0].month(), time::Month::March);

                // Null should map to UNIX Epoch (1970) as a safe fallback
                assert_eq!(data[1].year(), 1970);
                assert!(validity.expect("Mask missing")[0] & 0b11 == 0b01);
            }
        }

        #[test]
        fn test_arrow_empty_array() {
            let empty_f64 = Float64Array::from(Vec::<f64>::new());
            let col = ColumnVector::from_arrow(&empty_f64).unwrap();
            assert_eq!(col.len(), 0);
        }
    }
}
