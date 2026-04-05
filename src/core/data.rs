use crate::core::utils::{IntoParallelizable, Parallelizable};
use crate::error::ChartonError;
use ahash::{AHashMap, AHashSet};
use std::fmt;
use std::sync::Arc;
use time::OffsetDateTime;

#[cfg(feature = "arrow")]
use arrow::array::{Array, Float32Array, Float64Array, Int64Array, StringArray};
#[cfg(feature = "arrow")]
use arrow::datatypes::{DataType, TimeUnit};
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Encapsulates a single column of data with high-performance null handling.
///
/// Charton uses a columnar memory layout similar to Apache Arrow. Numerical
/// types are stored in contiguous vectors for GPU-friendly access, while
/// null values are tracked via bitmasks (validity maps) or IEEE 754 NaN values.
#[derive(Clone, Debug)]
pub enum ColumnVector {
    /// 64-bit floats. Nulls are represented by `f64::NAN` for zero-overhead hardware support.
    F64 { data: Vec<f64> },
    /// 32-bit floats. Nulls are represented by `f32::NAN`.
    F32 { data: Vec<f32> },
    /// 64-bit integers. Since integers lack a NaN state, nulls are tracked via `validity`.
    I64 {
        data: Vec<i64>,
        /// Bitmask where 1 = Valid, 0 = Null. If None, all rows are valid.
        validity: Option<Vec<u8>>,
    },
    /// 32-bit integers. Since integers lack a NaN state, nulls are tracked via `validity`.
    I32 {
        data: Vec<i32>,
        validity: Option<Vec<u8>>,
    },
    /// 32-bit unsigned integers. Commonly used for counts or discrete indices.
    U32 {
        data: Vec<u32>,
        validity: Option<Vec<u8>>,
    },
    /// String data. Nulls are stored as empty strings and tracked via `validity`.
    String {
        data: Vec<String>,
        validity: Option<Vec<u8>>,
    },
    /// Temporal data. Nulls are tracked via `validity`.
    DateTime {
        data: Vec<OffsetDateTime>,
        validity: Option<Vec<u8>>,
    },
}

/// Mapping raw types to semantic types allows the engine to automatically
/// select the appropriate Scale (Linear, Temporal, or Discrete) and validation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    /// Quantitative/Numeric data that supports arithmetic and interpolation (e.g., 1.2, 100).
    /// Maps to: LinearScale, LogScale.
    Continuous,

    /// Categorical or Qualitative data used for grouping or indexing (e.g., "Apple", "Orange").
    /// Maps to: DiscreteScale.
    Discrete,

    /// Time-based data represented as points in a timeline.
    /// Maps to: TimeScale.
    Temporal,
}

impl ColumnVector {
    /// Infers the [SemanticType] of the column based on its internal storage variant.
    ///
    /// This is a low-latency operation used to guide the selection of
    /// visual encoding strategies (e.g., choosing a TimeScale for DateTime).
    pub fn semantic_type(&self) -> SemanticType {
        match self {
            ColumnVector::F64 { .. }
            | ColumnVector::F32 { .. }
            | ColumnVector::I64 { .. }
            | ColumnVector::I32 { .. }
            | ColumnVector::U32 { .. } => SemanticType::Continuous,
            ColumnVector::String { .. } => SemanticType::Discrete,
            ColumnVector::DateTime { .. } => SemanticType::Temporal,
        }
    }

    /// Returns the number of rows in this column.
    pub fn len(&self) -> usize {
        match self {
            ColumnVector::F64 { data } => data.len(),
            ColumnVector::F32 { data } => data.len(),
            ColumnVector::I64 { data, .. } => data.len(),
            ColumnVector::I32 { data, .. } => data.len(),
            ColumnVector::U32 { data, .. } => data.len(),
            ColumnVector::String { data, .. } => data.len(),
            ColumnVector::DateTime { data, .. } => data.len(),
        }
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

    /// Safely retrieves a value as f64 for numerical calculations.
    ///
    /// This handles:
    /// 1. Type casting from I64, I32, U32, F32 to F64.
    /// 2. Null-checking via the validity bitmask.
    /// 3. NaN-checking for float types.
    pub fn get_f64(&self, row: usize) -> Option<f64> {
        match self {
            // Floating point types check for NaN internally
            ColumnVector::F64 { data } => {
                let v = data[row];
                if v.is_nan() { None } else { Some(v) }
            }
            ColumnVector::F32 { data } => {
                let v = data[row];
                if v.is_nan() { None } else { Some(v as f64) }
            }

            // Integer types check the validity bitmask
            ColumnVector::I64 { data, validity } => {
                if ColumnVector::is_valid_in_mask(validity, row) {
                    Some(data[row] as f64)
                } else {
                    None
                }
            }
            // Integer types check the validity bitmask
            ColumnVector::I32 { data, validity } => {
                if ColumnVector::is_valid_in_mask(validity, row) {
                    Some(data[row] as f64)
                } else {
                    None
                }
            }
            ColumnVector::U32 { data, validity } => {
                if ColumnVector::is_valid_in_mask(validity, row) {
                    Some(data[row] as f64)
                } else {
                    None
                }
            }

            // String and DateTime are not direct numerical types for this method
            _ => None,
        }
    }

    /// Alias for your existing get_f64 to match the calling code.
    pub fn get_as_f64(&self, row: usize) -> Option<f64> {
        self.get_f64(row)
    }

    /// Projects the entire column into a contiguous `f64` vector.
    ///
    /// This is a high-cost operation ($O(n)$ time + memory allocation),
    /// hence the `to_` prefix to signal ownership transfer and allocation.
    ///
    /// The logic is internally consistent with `get_f64`, ensuring that
    /// type casting and validity bitmask checks remain synchronized.
    pub fn to_f64_vec(&self) -> Vec<f64> {
        let n = self.len();
        let mut out = Vec::with_capacity(n);

        match self {
            // Optimized Path: If underlying data is already F64,
            // we bypass per-row enum dispatching and handle NaNs directly.
            ColumnVector::F64 { data } => {
                out.extend(data.iter().map(|&v| if v.is_nan() { 0.0 } else { v }));
            }
            // Optimized Path: Bulk conversion from F32 to F64.
            ColumnVector::F32 { data } => {
                out.extend(
                    data.iter()
                        .map(|&v| if v.is_nan() { 0.0 } else { v as f64 }),
                );
            }
            // Generic Path: Handles I64, I32, U32, and other numeric types
            // by utilizing the validity bitmask-aware logic in `get_f64`.
            _ => {
                for i in 0..n {
                    // Fallback to unified logic; maintains single-point-of-truth for null handling.
                    out.push(self.get_f64(i).unwrap_or(0.0));
                }
            }
        }
        out
    }

    /// Projects the column into a vector of `Option<f64>`, preserving the original
    /// null/validity states. Useful for statistical calculations where nulls
    /// should not be coerced to 0.0.
    pub fn to_f64_options(&self) -> Vec<Option<f64>> {
        (0..self.len()).map(|i| self.get_f64(i)).collect()
    }

    /// Retrieves a value as a String for grouping or labeling.
    /// This is used as the 'Key' in group-by operations (like stacking).
    pub fn get_as_string(&self, row: usize) -> Option<String> {
        match self {
            ColumnVector::String { data, validity } => {
                if Self::is_valid_in_mask(validity, row) {
                    Some(data[row].clone())
                } else {
                    None
                }
            }
            ColumnVector::I64 { data, validity } => {
                if Self::is_valid_in_mask(validity, row) {
                    Some(format!("{}", data[row]))
                } else {
                    None
                }
            }
            ColumnVector::I32 { data, validity } => {
                if Self::is_valid_in_mask(validity, row) {
                    Some(format!("{}", data[row]))
                } else {
                    None
                }
            }
            ColumnVector::U32 { data, validity } => {
                if Self::is_valid_in_mask(validity, row) {
                    Some(format!("{}", data[row]))
                } else {
                    None
                }
            }
            ColumnVector::F64 { data } => {
                let v = data[row];
                if v.is_nan() {
                    None
                } else {
                    Some(format!("{}", v))
                }
            }
            ColumnVector::F32 { data } => {
                let v = data[row];
                if v.is_nan() {
                    None
                } else {
                    Some(format!("{}", v))
                }
            }
            ColumnVector::DateTime { data, validity } => {
                if Self::is_valid_in_mask(validity, row) {
                    Some(format!("{}", data[row]))
                } else {
                    None
                }
            }
        }
    }

    /// Returns the number of unique non-null values in the column.
    ///
    /// This implementation respects the specific null-representation of each
    /// variant (NaN for floats, bitmasks for others) to ensure accurate statistics.
    pub fn n_unique(&self) -> usize {
        match self {
            // --- FLOAT PATHS (F64/F32) ---
            // Normalizes -0.0 and 0.0 to the same bit representation and filters NaNs.
            ColumnVector::F64 { data } => {
                data.maybe_par_iter()
                    .filter(|&&v| !v.is_nan())
                    .fold(
                        || AHashSet::new(),
                        |mut set, &v| {
                            // In IEEE 754, -0.0 == 0.0 is true
                            let norm = if v == 0.0 { 0.0 } else { v };
                            set.insert(norm.to_bits());
                            set
                        },
                    )
                    .reduce(
                        || AHashSet::new(),
                        |mut s1, s2| {
                            s1.extend(s2);
                            s1
                        },
                    )
                    .len()
            }

            ColumnVector::F32 { data } => data
                .maybe_par_iter()
                .filter(|&&v| !v.is_nan())
                .fold(
                    || AHashSet::new(),
                    |mut set, &v| {
                        let norm = if v == 0.0 { 0.0 } else { v };
                        set.insert(norm.to_bits());
                        set
                    },
                )
                .reduce(
                    || AHashSet::new(),
                    |mut s1, s2| {
                        s1.extend(s2);
                        s1
                    },
                )
                .len(),

            // --- STRING PATH ---
            // Uses the validity bitmask to skip null strings during parallel iteration.
            ColumnVector::String { data, validity } => (0..data.len())
                .maybe_into_par_iter()
                .fold(
                    || AHashSet::new(),
                    |mut set, i| {
                        if Self::is_valid_in_mask(validity, i) {
                            set.insert(&data[i]);
                        }
                        set
                    },
                )
                .reduce(
                    || AHashSet::new(),
                    |mut s1, s2| {
                        s1.extend(s2);
                        s1
                    },
                )
                .len(),

            // --- INTEGER PATHS (I64, I32, U32) ---
            // Efficiently processes primitive integers using thread-local sets.
            ColumnVector::I64 { data, validity } => (0..data.len())
                .maybe_into_par_iter()
                .fold(
                    || AHashSet::new(),
                    |mut set, i| {
                        if Self::is_valid_in_mask(validity, i) {
                            set.insert(data[i]);
                        }
                        set
                    },
                )
                .reduce(
                    || AHashSet::new(),
                    |mut s1, s2| {
                        s1.extend(s2);
                        s1
                    },
                )
                .len(),

            ColumnVector::I32 { data, validity } => (0..data.len())
                .maybe_into_par_iter()
                .fold(
                    || AHashSet::new(),
                    |mut set, i| {
                        if Self::is_valid_in_mask(validity, i) {
                            set.insert(data[i]);
                        }
                        set
                    },
                )
                .reduce(
                    || AHashSet::new(),
                    |mut s1, s2| {
                        s1.extend(s2);
                        s1
                    },
                )
                .len(),

            ColumnVector::U32 { data, validity } => (0..data.len())
                .maybe_into_par_iter()
                .fold(
                    || AHashSet::new(),
                    |mut set, i| {
                        if Self::is_valid_in_mask(validity, i) {
                            set.insert(data[i]);
                        }
                        set
                    },
                )
                .reduce(
                    || AHashSet::new(),
                    |mut s1, s2| {
                        s1.extend(s2);
                        s1
                    },
                )
                .len(),

            // --- TEMPORAL PATH ---
            ColumnVector::DateTime { data, validity } => (0..data.len())
                .maybe_into_par_iter()
                .fold(
                    || AHashSet::new(),
                    |mut set, i| {
                        if Self::is_valid_in_mask(validity, i) {
                            set.insert(data[i]);
                        }
                        set
                    },
                )
                .reduce(
                    || AHashSet::new(),
                    |mut s1, s2| {
                        s1.extend(s2);
                        s1
                    },
                )
                .len(),
        }
    }

    /// Returns a stable, unique list of values as Strings for Discrete scales.
    ///
    /// This is ONLY intended for Discrete (Categorical) axes. For performance,
    /// it only handles types that are commonly used as categories.
    pub fn unique_values(&self) -> Vec<String> {
        let mut result = Vec::new();

        match self {
            // String is the primary candidate for Discrete scales.
            ColumnVector::String { data, validity } => {
                let mut seen = AHashSet::new();
                for (i, s) in data.iter().enumerate() {
                    if Self::is_valid_in_mask(validity, i) {
                        if seen.insert(s) {
                            result.push(s.clone());
                        }
                    }
                }
            }

            // Integers can often act as Discrete categories (e.g., Year, ID, Group Number).
            ColumnVector::I64 { data, validity } => {
                let mut seen = AHashSet::new();
                for (i, &v) in data.iter().enumerate() {
                    if Self::is_valid_in_mask(validity, i) {
                        if seen.insert(v) {
                            result.push(v.to_string());
                        }
                    }
                }
            }

            // For I32/U32, the logic is identical.
            ColumnVector::I32 { data, validity } => {
                let mut seen = AHashSet::new();
                for (i, &v) in data.iter().enumerate() {
                    if Self::is_valid_in_mask(validity, i) {
                        if seen.insert(v) {
                            result.push(v.to_string());
                        }
                    }
                }
            }

            // We skip F64/F32 in unique_values.
            // If a user forces a Float column to be Discrete, they should
            // usually bin it first or cast it to String.
            _ => {
                // Return empty or a basic string representation if absolutely necessary,
                // but usually, SemanticType::Discrete won't be assigned to Floats.
            }
        }
        result
    }

    /// Computes both minimum and maximum values in a single parallel scan.
    ///
    /// Returns a tuple `(min, max)` as `f64`. This method handles null-checks
    /// (NaN for floats and bitmasks for other types) and uses Rayon for
    /// multi-threaded execution.
    pub fn min_max(&self) -> (f64, f64) {
        let identity = (f64::INFINITY, f64::NEG_INFINITY);

        match self {
            // --- FLOAT PATHS ---
            ColumnVector::F64 { data } => data
                .maybe_par_iter()
                .filter(|&&v| !v.is_nan())
                .fold(|| identity, |(min, max), &v| (min.min(v), max.max(v)))
                .reduce(|| identity, |(m1, x1), (m2, x2)| (m1.min(m2), x1.max(x2))),
            ColumnVector::F32 { data } => data
                .maybe_par_iter()
                .filter(|&&v| !v.is_nan())
                .fold(
                    || identity,
                    |(min, max), &v| {
                        let v64 = v as f64;
                        (min.min(v64), max.max(v64))
                    },
                )
                .reduce(|| identity, |(m1, x1), (m2, x2)| (m1.min(m2), x1.max(x2))),

            // --- INTEGER PATHS ---
            // Explicitly cast primitives to f64 via the closure.
            ColumnVector::I64 { data, validity } => {
                self.parallel_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::I32 { data, validity } => {
                self.parallel_scan_with_mask(data, validity, |&v| v as f64)
            }
            ColumnVector::U32 { data, validity } => {
                self.parallel_scan_with_mask(data, validity, |&v| v as f64)
            }

            // --- TEMPORAL PATH ---
            // Converts OffsetDateTime to a Unix timestamp (seconds) for numeric scaling.
            ColumnVector::DateTime { data, validity } => {
                self.parallel_scan_with_mask(data, validity, |&v| v.unix_timestamp() as f64)
            }

            // --- DISCRETE/OTHER ---
            _ => (0.0, 0.0),
        }
    }

    /// Internal parallel scanner utilizing a Map-Reduce pattern for maximum throughput.
    ///
    /// Takes a data slice, an optional validity mask, and a conversion closure.
    /// Fails gracefully by skipping masked (null) values.
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
        let identity = (f64::INFINITY, f64::NEG_INFINITY);

        if let Some(mask) = validity {
            data.maybe_par_iter()
                .enumerate()
                .fold(
                    || identity,
                    |(min, max), (i, v)| {
                        // Check the i-th bit in the bitmask
                        if (mask[i / 8] >> (i % 8)) & 1 == 1 {
                            let val = convert(v);
                            (min.min(val), max.max(val))
                        } else {
                            (min, max)
                        }
                    },
                )
                .reduce(|| identity, |(m1, x1), (m2, x2)| (m1.min(m2), x1.max(x2)))
        } else {
            // Optimization: No bitmask present, process all elements.
            data.maybe_par_iter()
                .fold(
                    || identity,
                    |(min, max), v| {
                        let val = convert(v);
                        (min.min(val), max.max(val))
                    },
                )
                .reduce(|| identity, |(m1, x1), (m2, x2)| (m1.min(m2), x1.max(x2)))
        }
    }
}

/// Internal trait to bridge ColumnVector and concrete Rust types.
/// Get data from a column vector.
pub trait FromColumnVector: Sized {
    fn try_from_col(col: &ColumnVector) -> Option<&[Self]>;
}

macro_rules! impl_from_col {
    ($t:ty, $variant:ident) => {
        impl FromColumnVector for $t {
            fn try_from_col(col: &ColumnVector) -> Option<&[Self]> {
                match col {
                    ColumnVector::$variant { data, .. } => Some(data),
                    _ => None,
                }
            }
        }
    };
}

impl_from_col!(f64, F64);
impl_from_col!(f32, F32);
impl_from_col!(i64, I64);
impl_from_col!(i32, I32);
impl_from_col!(u32, U32);
impl_from_col!(String, String);
impl_from_col!(OffsetDateTime, DateTime);

/// Represents the result of a grouping operation, preserving the order of appearance.
pub struct GroupedIndices {
    /// A vector of tuples where:
    /// - `Option<String>` is the group name (e.g., "North America").
    /// - `Vec<usize>` contains the **original row indices** belonging to that group.
    ///
    /// ### Order of Groups
    /// The groups are ordered by their **first appearance** in the original dataset.
    /// This allows users to control chart sorting by simply reordering their data source.
    pub groups: Vec<(Option<String>, Vec<usize>)>,
}

/// A normalized, columnar data container.
///
/// `Dataset` is the internal "Single Source of Truth" for Charton.
/// It decouples plotting logic from external data frame libraries.
#[derive(Clone)]
pub struct Dataset {
    pub(crate) schema: AHashMap<String, usize>,
    pub(crate) columns: Vec<Arc<ColumnVector>>,
    pub(crate) row_count: usize,
}

impl Dataset {
    pub fn new() -> Self {
        Self {
            schema: AHashMap::new(),
            columns: Vec::new(),
            row_count: 0,
        }
    }

    /// Internal helper to validate row length consistency across columns.
    fn validate_len(&mut self, name: &str, incoming_len: usize) -> Result<(), ChartonError> {
        if self.columns.is_empty() {
            self.row_count = incoming_len;
            Ok(())
        } else if incoming_len != self.row_count {
            Err(ChartonError::Data(format!(
                "Inconsistent column length in '{}': expected {} rows, found {}",
                name, self.row_count, incoming_len
            )))
        } else {
            Ok(())
        }
    }

    /// Adds a new column to the dataset (Imperative Style).
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

        // Ensure the new column matches the dataset's row count
        self.validate_len(&name_str, vec.len())?;

        let index = self.columns.len();
        self.columns.push(Arc::new(vec));
        self.schema.insert(name_str, index);
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
    pub fn height(&self) -> usize {
        self.row_count
    }

    /// Returns the number of columns in the dataset.
    pub fn width(&self) -> usize {
        self.columns.len()
    }

    /// Returns a list of all column names present in the dataset.
    ///
    /// This is useful for UI components or discovery logic to know
    /// which dimensions are available for encoding.
    pub fn get_column_names(&self) -> Vec<String> {
        // Since schema is a HashMap<String, usize>, we can just collect the keys.
        // Note: The order of names is not guaranteed due to HashMap's nature.
        self.schema.keys().cloned().collect()
    }

    /// Returns a reference to the [ColumnVector] wrapper for the specified column.
    ///
    /// This is the primary method for metadata inspection (type checking, null-mask access)
    /// without needing to know the underlying concrete type T.
    pub fn column(&self, name: &str) -> Result<&ColumnVector, ChartonError> {
        let index = self
            .schema
            .get(name)
            .ok_or_else(|| ChartonError::Data(format!("Column '{}' not found in dataset", name)))?;
        Ok(&self.columns[*index])
    }

    /// High-performance: Returns a reference to the entire column data.
    /// This is the preferred way for rendering and bulk calculations.
    pub fn get_column<T: FromColumnVector>(&self, name: &str) -> Result<&[T], ChartonError> {
        let index = self
            .schema
            .get(name)
            .ok_or_else(|| ChartonError::Data(format!("Column '{}' not found", name)))?;

        T::try_from_col(&self.columns[*index]).ok_or_else(|| {
            ChartonError::Data(format!(
                "Type mismatch: Column '{}' cannot be accessed as the requested type",
                name
            ))
        })
    }

    /// Interaction-focused: Returns a single value.
    /// Use this for tooltips or specific data inspections.
    pub fn get_value<T: FromColumnVector>(
        &self,
        name: &str,
        row: usize,
    ) -> Result<&T, ChartonError> {
        let data = self.get_column::<T>(name)?;
        data.get(row)
            .ok_or_else(|| ChartonError::Data(format!("Index {} out of bounds", row)))
    }

    /// Check if a value at a specific row is null (validity bit is 0).
    pub fn is_null(&self, name: &str, row: usize) -> bool {
        let index = match self.schema.get(name) {
            Some(i) => *i,
            None => return true,
        };

        // self.columns[index] is Arc<ColumnVector>
        match &*self.columns[index] {
            ColumnVector::F64 { data } => data[row].is_nan(),
            ColumnVector::F32 { data } => data[row].is_nan(),
            ColumnVector::I64 { validity, .. }
            | ColumnVector::I32 { validity, .. }
            | ColumnVector::U32 { validity, .. }
            | ColumnVector::String { validity, .. }
            | ColumnVector::DateTime { validity, .. } => {
                if let Some(v) = validity {
                    // Extract the specific bit: 0 means null
                    (v[row / 8] >> (row % 8)) & 1 == 0
                } else {
                    false // No validity map means 100% valid
                }
            }
        }
    }

    /// Validates a single column against a set of allowed [SemanticType]s.
    ///
    /// This prevents "illegal" mappings, such as trying to use a Categorical string
    /// for a Continuous 'Size' channel.
    pub fn validate_column_semantic(
        &self,
        column_name: &str,
        allowed: &[SemanticType],
    ) -> Result<SemanticType, ChartonError> {
        // Access the column vector reference to inspect its semantic type.
        let col = self.column(column_name)?;
        let actual = col.semantic_type();

        if !allowed.contains(&actual) {
            return Err(ChartonError::Data(format!(
                "Column '{}' (Semantic: {:?}) is incompatible. Expected one of: {:?}",
                column_name, actual, allowed
            )));
        }

        Ok(actual)
    }

    /// Performs a bulk validation of the dataset schema against encoding requirements.
    ///
    /// # Arguments
    /// * `required_columns` - Column names that must exist in the dataset.
    /// * `expected_semantics` - A map defining allowed semantic types for specific columns.
    ///
    /// # Returns
    /// A map of column names to their resolved [SemanticType]s for downstream Scale initialization.
    pub fn check_schema(
        &self,
        required_columns: &[&str],
        expected_semantics: &AHashMap<&str, Vec<SemanticType>>,
    ) -> Result<AHashMap<String, SemanticType>, ChartonError> {
        let mut resolved_semantics = AHashMap::new();

        for &col_name in required_columns {
            // Default to allowing all types if no specific constraint is provided for this column.
            let allowed = expected_semantics
                .get(col_name)
                .map(|v| v.as_slice())
                .unwrap_or(&[
                    SemanticType::Continuous,
                    SemanticType::Discrete,
                    SemanticType::Temporal,
                ]);

            let actual = self.validate_column_semantic(col_name, allowed)?;
            resolved_semantics.insert(col_name.to_string(), actual);
        }

        Ok(resolved_semantics)
    }

    /// Generates a combined bitmask for multiple columns.
    ///
    /// This is a high-performance "AND" operation across multiple validity maps.
    /// Use this before rendering to get a single 'view' of which rows are fully valid.
    pub fn get_combined_mask(&self, column_names: &[&str]) -> Result<Vec<u8>, ChartonError> {
        if self.row_count == 0 {
            return Ok(Vec::new());
        }

        // Start with all bits set to 1 (Valid)
        let byte_count = (self.row_count + 7) / 8;
        let mut final_mask = vec![0xFFu8; byte_count];

        for &name in column_names {
            let col = self.column(name)?;
            match col {
                ColumnVector::F64 { data } => {
                    for (i, val) in data.iter().enumerate() {
                        if val.is_nan() {
                            final_mask[i / 8] &= !(1 << (i % 8));
                        }
                    }
                }
                ColumnVector::F32 { data } => {
                    for (i, val) in data.iter().enumerate() {
                        if val.is_nan() {
                            final_mask[i / 8] &= !(1 << (i % 8));
                        }
                    }
                }
                ColumnVector::I64 { validity, .. }
                | ColumnVector::I32 { validity, .. }
                | ColumnVector::U32 { validity, .. }
                | ColumnVector::String { validity, .. }
                | ColumnVector::DateTime { validity, .. } => {
                    if let Some(v) = validity {
                        // Efficient bitwise AND across the entire byte vector
                        for (i, byte) in v.iter().enumerate() {
                            final_mask[i] &= byte;
                        }
                    }
                }
            }
        }

        // Clean trailing bits in the last byte
        if self.row_count % 8 != 0 {
            let last_idx = byte_count - 1;
            let mask = (1 << (self.row_count % 8)) - 1;
            final_mask[last_idx] &= mask;
        }

        Ok(final_mask)
    }

    /// Internal helper to convert a specific cell value into a string for display.
    /// Handles null checks, numerical precision, and string truncation.
    fn debug_cell(&self, col_name: &str, row: usize) -> String {
        // Check for missing data via NaN or Validity Bitmaps
        if self.is_null(col_name, row) {
            return "null".to_string();
        }

        let idx = *self.schema.get(col_name).expect("Schema integrity error");
        match &*self.columns[idx] {
            // Format floating points to 4 decimal places for readability
            ColumnVector::F64 { data } => format!("{:.4}", data[row]),
            ColumnVector::F32 { data } => format!("{:.4}", data[row]),

            // Standard integer to string conversion
            ColumnVector::I64 { data, .. } => data[row].to_string(),
            ColumnVector::I32 { data, .. } => data[row].to_string(),
            ColumnVector::U32 { data, .. } => data[row].to_string(),

            // Truncate long strings to keep the table layout neat
            ColumnVector::String { data, .. } => {
                let s = &data[row];
                // Check if the number of characters (not bytes) exceeds the limit
                if s.chars().count() > 10 {
                    // Safely find the byte index of the 7th character
                    let safe_index = s
                        .char_indices()
                        .nth(7)
                        .map(|(idx, _char)| idx)
                        .unwrap_or(s.len());

                    format!("{}...", &s[..safe_index])
                } else {
                    s.clone()
                }
            }

            // Format timestamps using the standard ISO 8601 (RFC 3339) format
            ColumnVector::DateTime { data, .. } => data[row]
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "err_date".to_string()),
        }
    }

    /// Partitions the dataset using aHash and Rayon for maximum throughput,
    /// while preserving the order of groups based on their first appearance.
    pub fn group_by(&self, col_name: Option<&str>) -> GroupedIndices {
        // 1. Resolve the grouping column.
        let col_vector = match col_name {
            Some(name) => self.column(name).ok(),
            None => None,
        };

        // 2. Handle the "No Grouping" case.
        // We return a Vec with a single group containing all indices.
        let vector = match col_vector {
            Some(v) => v,
            None => {
                return GroupedIndices {
                    groups: vec![(None, (0..self.row_count).collect())],
                };
            }
        };

        // 3. Parallel Grouping with First-Appearance Tracking:
        // We store: Map<GroupName, (FirstSeenIndex, Vec<RowIndices>)>
        let groups_map = (0..self.row_count)
            .into_par_iter()
            .fold(
                || AHashMap::<Option<String>, (usize, Vec<usize>)>::with_capacity(64),
                |mut local_map, i| {
                    let key = vector.get_as_string(i);
                    local_map
                        .entry(key)
                        .and_modify(|(_first_idx, indices)| {
                            // The local_map processes indices in increasing order,
                            // so first_idx is already the minimum for this chunk.
                            indices.push(i);
                        })
                        .or_insert((i, vec![i]));
                    local_map
                },
            )
            .reduce(
                || AHashMap::default(),
                |mut map1, mut map2| {
                    for (key, (first_idx2, mut indices2)) in map2.drain() {
                        map1.entry(key)
                            .and_modify(|(first_idx1, indices1)| {
                                // Keep the global minimum index to preserve data order
                                if first_idx2 < *first_idx1 {
                                    *first_idx1 = first_idx2;
                                }
                                indices1.append(&mut indices2);
                            })
                            .or_insert((first_idx2, indices2));
                    }
                    map1
                },
            );

        // 4. Finalize Order: Convert the HashMap to a Vec sorted by first_idx.
        let mut sorted_groups: Vec<(Option<String>, (usize, Vec<usize>))> =
            groups_map.into_iter().collect();

        // Sort groups based on when they first appeared in the data.
        // This allows users to control chart order via data sorting.
        sorted_groups.sort_by_key(|(_key, (first_idx, _indices))| *first_idx);

        // 5. Build the final GroupedIndices structure.
        let groups = sorted_groups
            .into_iter()
            .map(|(key, (_first_idx, mut indices))| {
                // Sort indices within the group to ensure contiguous memory access
                // patterns during the rendering phase.
                indices.sort_unstable();
                (key, indices)
            })
            .collect();

        GroupedIndices { groups }
    }
}

/// Enable printing of Dataset
impl fmt::Debug for Dataset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print basic metadata about the dataset dimensions
        writeln!(
            f,
            "Dataset: {} rows x {} columns",
            self.row_count,
            self.columns.len()
        )?;

        // 1. Organize headers sorted by their internal column index
        let mut names: Vec<_> = self.schema.keys().collect();
        names.sort_by_key(|name| self.schema.get(*name).unwrap());

        // 2. Format and print the header row with fixed-width alignment
        let header = names
            .iter()
            .map(|n| format!("{:<12}", n))
            .collect::<Vec<_>>()
            .join("| ");
        writeln!(f, "{}", header)?;
        writeln!(f, "{}", "-".repeat(header.len()))?;

        // 3. Print the first 10 rows to prevent console overflow on large datasets
        let limit = self.row_count.min(10);
        for row in 0..limit {
            let mut row_str = Vec::new();
            for name in &names {
                // Transpose columnar data into a row-wise string representation
                let cell = self.debug_cell(name, row);
                row_str.push(format!("{:<12}", cell));
            }
            writeln!(f, "{}", row_str.join("| "))?;
        }

        // Indicate if there is more data beyond the displayed rows
        if self.row_count > 10 {
            writeln!(f, "... and {} more rows", self.row_count - 10)?;
        }

        Ok(())
    }
}

// --- Conversion Implementations from Option-based Vectors ---

/// Helper function to create a validity bitmask from an iterator of Options.
/// Returns (DataVec, ValidityMask).
///
/// The `T: Clone` bound is required to fill "null" slots with a default value.
fn collect_with_validity<T, I>(iter: I, default: T) -> (Vec<T>, Option<Vec<u8>>)
where
    I: IntoIterator<Item = Option<T>>,
    T: Clone, // Add the trait bound here
{
    let iter = iter.into_iter();
    let (lower, _) = iter.size_hint();
    let mut data = Vec::with_capacity(lower);

    // Each u8 stores 8 rows of validity bits.
    let mut validity = Vec::with_capacity((lower + 7) / 8);
    let mut has_nulls = false;

    let mut current_byte = 0u8;
    let mut bit_count = 0;

    for opt in iter {
        match opt {
            Some(v) => {
                data.push(v);
                // Set the corresponding bit to 1 (Valid)
                current_byte |= 1 << (bit_count % 8);
            }
            None => {
                // Fill the gap with the default value (e.g., 0 or "")
                data.push(default.clone());
                has_nulls = true;
                // The bit remains 0 (Null)
            }
        }

        bit_count += 1;
        // If we've filled 8 bits, push the byte and reset
        if bit_count % 8 == 0 {
            validity.push(current_byte);
            current_byte = 0;
        }
    }

    // Don't forget the last partial byte
    if bit_count % 8 != 0 {
        validity.push(current_byte);
    }

    // Optimization: If no None was ever encountered, discard the validity mask to save memory.
    (data, if has_nulls { Some(validity) } else { None })
}

// --- F64: Use NaN for Nulls (No Bitmask needed) ---
impl From<Vec<Option<f64>>> for ColumnVector {
    fn from(v: Vec<Option<f64>>) -> Self {
        let data = v.into_iter().map(|opt| opt.unwrap_or(f64::NAN)).collect();
        ColumnVector::F64 { data }
    }
}

// --- F32: Use NaN for Nulls (No Bitmask needed) ---
impl From<Vec<Option<f32>>> for ColumnVector {
    fn from(v: Vec<Option<f32>>) -> Self {
        let data = v.into_iter().map(|opt| opt.unwrap_or(f32::NAN)).collect();
        ColumnVector::F32 { data }
    }
}

// --- I64: Use Bitmask for Nulls ---
impl From<Vec<Option<i64>>> for ColumnVector {
    fn from(v: Vec<Option<i64>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0i64);
        ColumnVector::I64 { data, validity }
    }
}

// --- I32: Use Bitmask for Nulls ---
impl From<Vec<Option<i32>>> for ColumnVector {
    fn from(v: Vec<Option<i32>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0i32);
        ColumnVector::I32 { data, validity }
    }
}

// --- U32: Use Bitmask for Nulls ---
impl From<Vec<Option<u32>>> for ColumnVector {
    fn from(v: Vec<Option<u32>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0u32);
        ColumnVector::U32 { data, validity }
    }
}

// --- String1: For owned Strings ---
impl From<Vec<Option<String>>> for ColumnVector {
    fn from(v: Vec<Option<String>>) -> Self {
        let (data, validity) = collect_with_validity(v, String::new());
        ColumnVector::String { data, validity }
    }
}

// --- String2 For borrowed string slices (&str) ---
// Note: We use 'static or a generic lifetime, but usually 'static is enough for literals
impl From<Vec<Option<&str>>> for ColumnVector {
    fn from(v: Vec<Option<&str>>) -> Self {
        // Convert &str to String during collection
        let (data, validity) = collect_with_validity(
            v.into_iter().map(|opt| opt.map(|s| s.to_string())),
            String::new(),
        );
        ColumnVector::String { data, validity }
    }
}

// --- DateTime: Use Bitmask ---
impl From<Vec<Option<OffsetDateTime>>> for ColumnVector {
    fn from(v: Vec<Option<OffsetDateTime>>) -> Self {
        let (data, validity) = collect_with_validity(v, OffsetDateTime::UNIX_EPOCH);
        ColumnVector::DateTime { data, validity }
    }
}

// --- Support for Non-Option Vectors (Assume 100% validity) ---
impl From<Vec<f64>> for ColumnVector {
    fn from(data: Vec<f64>) -> Self {
        ColumnVector::F64 { data }
    }
}

impl From<Vec<f32>> for ColumnVector {
    fn from(data: Vec<f32>) -> Self {
        ColumnVector::F32 { data }
    }
}

impl From<Vec<i64>> for ColumnVector {
    fn from(data: Vec<i64>) -> Self {
        ColumnVector::I64 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<i32>> for ColumnVector {
    fn from(data: Vec<i32>) -> Self {
        ColumnVector::I32 {
            data,
            validity: None,
        }
    }
}

impl From<Vec<u32>> for ColumnVector {
    fn from(data: Vec<u32>) -> Self {
        ColumnVector::U32 {
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
        let data = v.into_iter().map(|s| s.to_string()).collect();
        ColumnVector::String {
            data,
            validity: None,
        }
    }
}

// --- DateTime: Standard Vector (100% Valid) ---
impl From<Vec<OffsetDateTime>> for ColumnVector {
    fn from(data: Vec<OffsetDateTime>) -> Self {
        // We skip the bitmask entirely to save memory and CPU cycles
        ColumnVector::DateTime {
            data,
            validity: None,
        }
    }
}

#[cfg(feature = "arrow")]
impl ColumnVector {
    /// Converts an Apache Arrow Array into a Charton ColumnVector.
    pub fn from_arrow(array: &dyn Array) -> Result<Self, ChartonError> {
        match array.data_type() {
            DataType::Float64 => {
                let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
                // Map nulls to NaN directly for floating point performance.
                let data: Vec<f64> = (0..arr.len())
                    .map(|i| {
                        if arr.is_null(i) {
                            f64::NAN
                        } else {
                            arr.value(i)
                        }
                    })
                    .collect();
                Ok(ColumnVector::F64 { data })
            }
            DataType::Float32 => {
                let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
                let data: Vec<f32> = (0..arr.len())
                    .map(|i| {
                        if arr.is_null(i) {
                            f32::NAN
                        } else {
                            arr.value(i)
                        }
                    })
                    .collect();
                Ok(ColumnVector::F32 { data })
            }
            DataType::Int64 => {
                let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
                // Reuse collect_with_validity by creating an iterator of Option<i64>
                let (data, validity) = collect_with_validity(
                    (0..arr.len()).map(|i| {
                        if arr.is_valid(i) {
                            Some(arr.value(i))
                        } else {
                            None
                        }
                    }),
                    0i64,
                );
                Ok(ColumnVector::I64 { data, validity })
            }
            DataType::Utf8 | DataType::LargeUtf8 => {
                let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
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
            DataType::Timestamp(unit, _) => {
                let (data, validity) = match unit {
                    TimeUnit::Second => {
                        let arr = array
                            .as_any()
                            .downcast_ref::<arrow::array::TimestampSecondArray>()
                            .unwrap();
                        collect_with_validity(
                            (0..arr.len()).map(|i| {
                                if arr.is_valid(i) {
                                    Some(
                                        OffsetDateTime::from_unix_timestamp(arr.value(i))
                                            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                                    )
                                } else {
                                    None
                                }
                            }),
                            OffsetDateTime::UNIX_EPOCH,
                        )
                    }
                    TimeUnit::Millisecond => {
                        let arr = array
                            .as_any()
                            .downcast_ref::<arrow::array::TimestampMillisecondArray>()
                            .unwrap();
                        collect_with_validity(
                            (0..arr.len()).map(|i| {
                                if arr.is_valid(i) {
                                    Some(
                                        OffsetDateTime::from_unix_timestamp_nanos(
                                            arr.value(i) as i128 * 1_000_000,
                                        )
                                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                                    )
                                } else {
                                    None
                                }
                            }),
                            OffsetDateTime::UNIX_EPOCH,
                        )
                    }
                    TimeUnit::Microsecond => {
                        let arr = array
                            .as_any()
                            .downcast_ref::<arrow::array::TimestampMicrosecondArray>()
                            .unwrap();
                        collect_with_validity(
                            (0..arr.len()).map(|i| {
                                if arr.is_valid(i) {
                                    Some(
                                        OffsetDateTime::from_unix_timestamp_nanos(
                                            arr.value(i) as i128 * 1_000,
                                        )
                                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                                    )
                                } else {
                                    None
                                }
                            }),
                            OffsetDateTime::UNIX_EPOCH,
                        )
                    }
                    TimeUnit::Nanosecond => {
                        let arr = array
                            .as_any()
                            .downcast_ref::<arrow::array::TimestampNanosecondArray>()
                            .unwrap();
                        collect_with_validity(
                            (0..arr.len()).map(|i| {
                                if arr.is_valid(i) {
                                    Some(
                                        OffsetDateTime::from_unix_timestamp_nanos(
                                            arr.value(i) as i128
                                        )
                                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                                    )
                                } else {
                                    None
                                }
                            }),
                            OffsetDateTime::UNIX_EPOCH,
                        )
                    }
                };

                Ok(ColumnVector::DateTime { data, validity })
            }
            _ => Err(ChartonError::Data(format!(
                "Unsupported Arrow type: {:?}",
                array.data_type()
            ))),
        }
    }
}

/// A convenience trait to improve the ergonomics of manual data construction.
///
/// This trait provides the `.into_column()` method for any type that can be
/// converted into a `ColumnVector`. It makes batch ingestion (like using
/// `to_dataset`) more readable by being explicit about the target type.
pub trait IntoColumn {
    /// Consumes the collection and converts it into a `ColumnVector`.
    fn into_column(self) -> ColumnVector;
}

/// Blanket implementation for any type that satisfies the `Into<ColumnVector>` bound.
///
/// This ensures that all our `From<Vec<T>>` implementations for `ColumnVector`
/// automatically gain the `.into_column()` method.
impl<T> IntoColumn for T
where
    T: Into<ColumnVector>,
{
    #[inline]
    fn into_column(self) -> ColumnVector {
        self.into()
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

            AggregateOp::Sum => indices.iter().filter_map(|&i| col.get_f64(i)).sum(),

            AggregateOp::Mean => {
                let mut sum = 0.0;
                let mut count = 0;
                for &i in indices {
                    if let Some(v) = col.get_f64(i) {
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
                .filter_map(|&i| col.get_f64(i))
                .fold(f64::INFINITY, f64::min),

            AggregateOp::Max => indices
                .iter()
                .filter_map(|&i| col.get_f64(i))
                .fold(f64::NEG_INFINITY, f64::max),

            // --- Median Implementation ---
            AggregateOp::Median => {
                let vals = self.extract_and_sort(col, indices);
                get_quantile(&vals, 0.5)
            }
        }
    }

    /// Extracts valid f64 values from the column at specified indices and sorts them in ascending order for median and quantile calculations.
    fn extract_and_sort(&self, col: &ColumnVector, indices: &[usize]) -> Vec<f64> {
        let mut vals: Vec<f64> = indices.iter().filter_map(|&i| col.get_f64(i)).collect();
        vals.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
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
        assert_eq!(*ds_from_tuples.get_value::<String>("name", 0).unwrap(), "A");

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
        assert_eq!(*ds.get_value::<i32>("id", 0).unwrap(), 100);
        assert!(!ds.is_null("id", 0));

        // Check row 1 (Null)
        // Note: get_value still returns a reference to the underlying data (likely 0),
        // so is_null is the authoritative way to check validity.
        assert!(ds.is_null("id", 1));

        // Check row 2 (Valid)
        assert_eq!(*ds.get_value::<i32>("id", 2).unwrap(), 300);

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
        // Specific imports for building Arrow arrays in tests
        use arrow::array::{Float64Array, Int64Array, StringArray, TimestampMillisecondArray};

        #[test]
        fn test_arrow_ingestion() {
            // 1. Test Float64 with Nulls (should become NaN for GPU/Canvas friendliness)
            let f64_array = Float64Array::from(vec![Some(1.1), None, Some(3.3)]);
            let col_f64 = ColumnVector::from_arrow(&f64_array).expect("F64 ingestion failed");

            if let ColumnVector::F64 { data } = col_f64 {
                println!("F64 Data (converted): {:?}", data);
                assert_eq!(data[0], 1.1);
                assert!(data[1].is_nan()); // Verify Null mapping
                assert_eq!(data[2], 3.3);
            }

            // 2. Test Int64 with Nulls (Verifying the validity bitmask)
            let i64_array = Int64Array::from(vec![Some(10), None, Some(30)]);
            let col_i64 = ColumnVector::from_arrow(&i64_array).expect("I64 ingestion failed");

            if let ColumnVector::I64 { data, validity } = col_i64 {
                println!("I64 Data: {:?}, Validity Mask: {:?}", data, validity);
                assert_eq!(data, vec![10, 0, 30]);
                assert!(validity.is_some());
                // Bitwise check: 0b101 (Index 0 valid, 1 invalid, 2 valid)
                assert_eq!(validity.unwrap()[0], 0b101);
            }

            // 3. Test StringArray
            let str_array = StringArray::from(vec![Some("Charton"), None, Some("Rust")]);
            let col_str = ColumnVector::from_arrow(&str_array).expect("String ingestion failed");

            if let ColumnVector::String { data, validity } = col_str {
                println!("String Data: {:?}, Validity Mask: {:?}", data, validity);
                assert_eq!(data[0], "Charton");
                assert_eq!(data[1], ""); // Default filler for strings
                assert_eq!(data[2], "Rust");
                assert!(validity.is_some());
            }

            // 4. Test Timestamp (Millisecond) - Verifying the i128 multiplier logic
            // 1711872000000 ms is 2024-03-31T08:00:00Z
            let ts_array = TimestampMillisecondArray::from(vec![Some(1711872000000), None]);
            let col_ts = ColumnVector::from_arrow(&ts_array).expect("Timestamp ingestion failed");

            if let ColumnVector::DateTime { data, validity } = col_ts {
                println!("DateTime Data: {:?}, Validity Mask: {:?}", data, validity);

                // Check if our multiplier correctly resulted in the year 2024
                assert_eq!(data[0].year(), 2024);
                assert_eq!(data[0].month(), time::Month::March);

                // Verify the null became UNIX_EPOCH (1970)
                assert_eq!(data[1].year(), 1970);
                assert!(validity.is_some());
            }
        }
    }
}
