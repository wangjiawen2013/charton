use std::collections::HashMap;
use crate::error::ChartonError;
use time::OffsetDateTime;

/// Encapsulates a single column of data with high-performance null handling.
/// 
/// Charton uses a columnar memory layout similar to Apache Arrow. Numerical 
/// types are stored in contiguous vectors for GPU-friendly access, while 
/// null values are tracked via bitmasks (validity maps) or IEEE 754 NaN values.
pub enum ColumnVector {
    /// 64-bit floats. Nulls are represented by `f64::NAN` for zero-overhead hardware support.
    F64 {
        data: Vec<f64>,
    },
    /// 32-bit floats. Nulls are represented by `f32::NAN`.
    F32 {
        data: Vec<f32>,
    },
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
        V: Into<ColumnVector>
    {
        let name_str = name.into();
        let vec: ColumnVector = data.into();
        self.validate_len(&name_str, vec.len())?;

        let index = self.columns.len();
        self.columns.push(vec);
        self.schema.insert(name_str, index);
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

// --- I64: Use Bitmask for Nulls ---
impl From<Vec<Option<i64>>> for ColumnVector {
    fn from(v: Vec<Option<i64>>) -> Self {
        let (data, validity) = collect_with_validity(v, 0i64);
        ColumnVector::I64 { data, validity }
    }
}

// --- String: Use Bitmask and Empty Strings ---
impl From<Vec<Option<String>>> for ColumnVector {
    fn from(v: Vec<Option<String>>) -> Self {
        let (data, validity) = collect_with_validity(v, String::new());
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
    fn from(data: Vec<f64>) -> Self { ColumnVector::F64 { data } }
}
impl From<Vec<i64>> for ColumnVector {
    fn from(data: Vec<i64>) -> Self { ColumnVector::I64 { data, validity: None } }
}
impl From<Vec<String>> for ColumnVector {
    fn from(data: Vec<String>) -> Self { ColumnVector::String { data, validity: None } }
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