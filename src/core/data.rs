use crate::error::ChartonError;
use std::collections::HashMap;
use std::fmt;
use time::OffsetDateTime;

/// Encapsulates a single column of data with high-performance null handling.
///
/// Charton uses a columnar memory layout similar to Apache Arrow. Numerical
/// types are stored in contiguous vectors for GPU-friendly access, while
/// null values are tracked via bitmasks (validity maps) or IEEE 754 NaN values.
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

impl ColumnVector {
    /// Returns the number of rows in this column.
    pub fn len(&self) -> usize {
        match self {
            ColumnVector::F64 { data } => data.len(),
            ColumnVector::F32 { data } => data.len(),
            ColumnVector::I64 { data, .. } => data.len(),
            ColumnVector::String { data, .. } => data.len(),
            ColumnVector::DateTime { data, .. } => data.len(),
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
impl_from_col!(String, String);
impl_from_col!(OffsetDateTime, DateTime);

/// A normalized, columnar data container.
///
/// `Dataset` is the internal "Single Source of Truth" for Charton.
/// It decouples plotting logic from external data frame libraries.
pub struct Dataset {
    pub(crate) schema: HashMap<String, usize>,
    pub(crate) columns: Vec<ColumnVector>,
    pub(crate) row_count: usize,
}

impl Dataset {
    pub fn new() -> Self {
        Self {
            schema: HashMap::new(),
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

    /// Adds a new column to the dataset.
    pub fn add_column<S, V>(&mut self, name: S, data: V) -> Result<(), ChartonError>
    where
        S: Into<String>,
        V: Into<ColumnVector>,
    {
        let name_str = name.into();
        let vec: ColumnVector = data.into();
        self.validate_len(&name_str, vec.len())?;

        let index = self.columns.len();
        self.columns.push(vec);
        self.schema.insert(name_str, index);
        Ok(())
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

        match &self.columns[index] {
            ColumnVector::F64 { data } => data[row].is_nan(),
            ColumnVector::F32 { data } => data[row].is_nan(),
            ColumnVector::I64 { validity, .. }
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

impl Dataset {
    /// Internal helper to convert a specific cell value into a string for display.
    /// Handles null checks, numerical precision, and string truncation.
    fn debug_cell(&self, col_name: &str, row: usize) -> String {
        // Check for missing data via NaN or Validity Bitmaps
        if self.is_null(col_name, row) {
            return "null".to_string();
        }

        let idx = *self.schema.get(col_name).expect("Schema integrity error");
        match &self.columns[idx] {
            // Format floating points to 4 decimal places for readability
            ColumnVector::F64 { data } => format!("{:.4}", data[row]),
            ColumnVector::F32 { data } => format!("{:.4}", data[row]),

            // Standard integer to string conversion
            ColumnVector::I64 { data, .. } => data[row].to_string(),

            // Truncate long strings to keep the table layout neat
            ColumnVector::String { data, .. } => {
                let s = &data[row];
                if s.len() > 10 {
                    format!("{}...", &s[..7])
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(*ds.get_value::<i64>("id", 0).unwrap(), 100);
        assert!(!ds.is_null("id", 0));

        // Check row 1 (Null)
        // Note: get_value still returns a reference to the underlying data (likely 0),
        // so is_null is the authoritative way to check validity.
        assert!(ds.is_null("id", 1));

        // Check row 2 (Valid)
        assert_eq!(*ds.get_value::<i64>("id", 2).unwrap(), 300);

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
    }
}
