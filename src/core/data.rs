use crate::error::ChartonError;
use polars::datatypes::DataType;
use polars::prelude::*;
use std::collections::HashMap;
use std::io::Cursor;

/// A bridge trait that defines how data enters Charton.
/// It allows us to support local DataFrames (0.49) and external versions (via Parquet).
pub trait IntoChartonSource {
    fn into_source(self) -> Result<DataFrameSource, ChartonError>;
}

/// A wrapper around a Polars DataFrame that provides plotting-specific functionality.
///
/// This struct serves as a data source for plotting operations, encapsulating a
/// Polars DataFrame and providing convenient methods for data access and manipulation
/// needed for visualization purposes.
///
/// # Type Parameters
///
/// None
///
/// # Fields
///
/// * `df` - The underlying Polars DataFrame containing the actual data
///
/// # Examples
///
/// ```rust,ignore
/// use polars::prelude::*;
/// use charton::data::DataFrameSource;
///
/// // Create a DataFrame
/// let df = df!("x" => &[1, 2, 3], "y" => &[4, 5, 6]).unwrap();
///
/// // Wrap it in a DataFrameSource
/// let data_source = DataFrameSource::new(&df);
/// ```
#[derive(Clone)]
pub struct DataFrameSource {
    pub(crate) df: DataFrame,
}

impl DataFrameSource {
    // Creates a new `DataFrameSource` instance from a given `DataFrame`.
    pub(crate) fn new(df: DataFrame) -> Self {
        Self { df }
    }

    /// The "Universal Port": Deserializes Parquet bytes into the library's Polars version.
    /// This is the key to cross-version compatibility.
    pub fn from_parquet_bytes(bytes: &[u8]) -> Result<Self, ChartonError> {
        let cursor = Cursor::new(bytes);
        let df = ParquetReader::new(cursor).finish()?;
        Ok(Self::new(df))
    }

    // Retrieves column data by name and returns a Series object
    pub(crate) fn column(&self, name: &str) -> Result<Series, ChartonError> {
        // 1. Get the Column object. In Polars 0.49, df.column(name) returns Result<&Column, PolarsError>
        let col = self.df.column(name)?;

        // 2. Use as_materialized_series() to ensure a Series reference is obtained.
        //    This is more robust than as_series(), as it handles internal Column structures
        //    that might not be recognized as a standard Series (e.g., single-row optimization).
        let series = col.as_materialized_series();

        // 3. Clone the Series and return it
        Ok(series.clone())
    }
}

// Support for native &DataFrame (0.49)
impl IntoChartonSource for &DataFrame {
    fn into_source(self) -> Result<DataFrameSource, ChartonError> {
        Ok(DataFrameSource::new(self.clone()))
    }
}

// Support for native &LazyFrame (0.49)
impl IntoChartonSource for &LazyFrame {
    fn into_source(self) -> Result<DataFrameSource, ChartonError> {
        // .collect() consumes the LazyFrame, so we clone the reference first
        let df = self.clone().collect()?;
        Ok(DataFrameSource::new(df))
    }
}

// Support for the Parquet bridge through array of bytes
impl IntoChartonSource for &[u8] {
    fn into_source(self) -> Result<DataFrameSource, ChartonError> {
        DataFrameSource::from_parquet_bytes(self)
    }
}

// Support for the Parquet bridge through Vec of bytes
// Note: we cannot use from_parquet_bytes here without unknown reasons
impl IntoChartonSource for &Vec<u8> {
    fn into_source(self) -> Result<DataFrameSource, ChartonError> {
        let cursor = Cursor::new(self);
        let df = ParquetReader::new(cursor).finish()?;
        Ok(DataFrameSource::new(df))
    }
}

impl IntoChartonSource for Vec<u8> {
    fn into_source(self) -> Result<DataFrameSource, ChartonError> {
        DataFrameSource::from_parquet_bytes(self.as_slice())
    }
}

// A helper function to convert numeric columns to f64
pub(crate) fn convert_numeric_types(
    df_source: DataFrameSource,
) -> Result<DataFrameSource, ChartonError> {
    let mut new_columns = Vec::new();

    for col in df_source.df.get_columns() {
        use polars::datatypes::DataType::*;
        match col.dtype() {
            UInt8 | UInt16 | UInt32 | UInt64 | Int8 | Int16 | Int32 | Int64 | Int128 | Float32
            | Float64 => {
                let casted = col.cast(&Float64)?;
                new_columns.push(casted);
            }
            _ => {
                new_columns.push(col.clone());
            }
        }
    }

    let new_df = DataFrame::new(new_columns)?;

    Ok(DataFrameSource::new(new_df))
}

/// Represents the high-level semantic category of a data column.
///
/// We use Continuous/Discrete to distinguish between data that requires
/// interpolation (gradients/linear scales) and data that requires
/// indexing (palettes/point scales).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    /// Quantitative/Numeric data (e.g., 1.2, 5.5, 100.0).
    Continuous,

    /// Categorical/Qualitative data (e.g., "Apple", "Orange", true/false).
    Discrete,

    /// Time-based data (e.g., 2026-01-21).
    Temporal,
}

/// Interprets the Polars [DataType] into a [SemanticType].
pub(crate) fn interpret_semantic_type(dtype: &DataType) -> SemanticType {
    match dtype {
        // Numeric types -> Continuous
        DataType::Float32
        | DataType::Float64
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt32
        | DataType::UInt64 => SemanticType::Continuous,

        // Time types -> Temporal
        DataType::Date | DataType::Datetime(_, _) | DataType::Time => SemanticType::Temporal,

        // String, Categorical, Boolean -> Discrete
        DataType::String | DataType::Categorical(_, _) | DataType::Boolean => {
            SemanticType::Discrete
        }

        // Fallback for everything else (List, Struct, Null)
        _ => SemanticType::Discrete,
    }
}

/// Validates that the DataFrame contains the required columns and that their
/// data types align with the expected semantic categories (Continuous, Discrete, Temporal).
///
/// This validation ensures that aesthetic mappings (e.g., color, size) are mathematically
/// compatible with the underlying data. For instance, a 'size' encoding typically
/// requires 'Continuous' data to map values to pixel diameters.
///
/// # Arguments
/// * `df` - The Polars DataFrame to validate.
/// * `required_columns` - A list of column names that must be present in the data.
/// * `expected_semantics` - A map linking column names to their allowed [SemanticType]s.
///
/// # Returns
/// * `Ok(())` if all requirements are met.
/// * `Err(ChartonError::Encoding)` if a column is missing.
/// * `Err(ChartonError::Data)` if a column's semantic type is incompatible.
pub(crate) fn check_schema(
    df: &mut DataFrame,
    required_columns: &[&str],
    expected_semantics: &HashMap<&str, Vec<SemanticType>>,
) -> Result<(), ChartonError> {
    let schema = df.schema();

    for &col_name in required_columns {
        // 1. Ensure the column exists in the current schema
        let actual_dtype = schema.get(col_name).ok_or_else(|| {
            ChartonError::Encoding(format!("Column '{}' not found in DataFrame", col_name))
        })?;

        // 2. Map the low-level Polars DataType to our high-level SemanticType
        let actual_semantic = interpret_semantic_type(actual_dtype);

        // 3. If semantic constraints are provided for this column, validate them
        if let Some(allowed_semantics) = expected_semantics.get(col_name)
            && !allowed_semantics.contains(&actual_semantic)
        {
            return Err(ChartonError::Data(format!(
                "Column '{}' (Type: {:?}) is categorized as {:?}, but expected one of {:?}",
                col_name, actual_dtype, actual_semantic, allowed_semantics
            )));
        }
    }

    Ok(())
}

/// Represents the statistical aggregation operations available for data transformation.
///
/// This enum defines how multiple data points are collapsed into a single value
/// during the transformation phase (e.g., in Bar or ErrorBar charts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AggregateOp {
    /// Total sum of all values in the group. Default for most charts.
    #[default]
    Sum,
    /// Arithmetic mean (average).
    Mean,
    /// The middle value in the sorted data set.
    Median,
    /// The smallest value in the group.
    Min,
    /// The largest value in the group.
    Max,
    /// The total count of data records in the group.
    Count,
}

impl AggregateOp {
    /// Converts the aggregation operator directly into a Polars expression.
    ///
    /// By centralizing this in `data.rs`, any component with access to the
    /// data source can consistently apply statistical summaries to a field.
    pub fn into_expr(&self, field: &str) -> Expr {
        match self {
            Self::Sum => col(field).sum(),
            Self::Mean => col(field).mean(),
            Self::Median => col(field).median(),
            Self::Min => col(field).min(),
            Self::Max => col(field).max(),
            Self::Count => col(field).count(),
        }
    }
}

/// Provides a convenient way to convert string literals into `AggregateOp`.
/// This enables the "String-based API" for end-users (e.g., .aggregate("mean")).
impl From<&str> for AggregateOp {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mean" | "avg" => Self::Mean,
            "sum" => Self::Sum,
            "min" => Self::Min,
            "max" => Self::Max,
            "count" | "n" => Self::Count,
            "median" => Self::Median,
            // Fallback to default Sum for unrecognized strings
            _ => Self::Sum,
        }
    }
}
