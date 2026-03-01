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
            && !allowed_semantics.contains(&actual_semantic) {
                return Err(ChartonError::Data(format!(
                    "Column '{}' (Type: {:?}) is categorized as {:?}, but expected one of {:?}",
                    col_name, actual_dtype, actual_semantic, allowed_semantics
                )));
            }
    }

    Ok(())
}

/// Load built-in datasets.
///
/// Based on the passed dataset name `dataset`, returns the corresponding `DataFrame` or an error.
///
/// # Arguments
///
/// - `dataset`: Dataset name, supporting the following options:
///   - `"mtcars"`: Motor trend car road tests dataset (32 rows × 12 columns)
///   - `"iris"`: Edgar Anderson's Iris Data (150 rows × 5 columns)
///   - `"mpg"`: Subset of first 10 rows from UCI Auto MPG dataset
///   - `"nightingale"`: Dataset for Florence Nightingale's famous polar area diagram.
///
/// # Returns
///
/// On success, returns a `DataFrame` containing the specified dataset content;
/// on failure, returns a `ChartonError`.
///
/// # Examples
///
/// ```rust,ignore
/// let df = load_dataset("mtcars")?;
/// ```
pub fn load_dataset(dataset: &str) -> Result<DataFrame, ChartonError> {
    match dataset {
        // mtcars dataset: Classic automobile performance dataset from https://www.kaggle.com/datasets/ruiromanini/mtcars
        "mtcars" => {
            // Car model column (string type)
            let model = vec![
                "Mazda RX4",
                "Mazda RX4 Wag",
                "Datsun 710",
                "Hornet 4 Drive",
                "Hornet Sportabout",
                "Valiant",
                "Duster 360",
                "Merc 240D",
                "Merc 230",
                "Merc 280",
                "Merc 280C",
                "Merc 450SE",
                "Merc 450SL",
                "Merc 450SLC",
                "Cadillac Fleetwood",
                "Lincoln Continental",
                "Chrysler Imperial",
                "Fiat 128",
                "Honda Civic",
                "Toyota Corolla",
                "Toyota Corona",
                "Dodge Challenger",
                "AMC Javelin",
                "Camaro Z28",
                "Pontiac Firebird",
                "Fiat X1-9",
                "Porsche 914-2",
                "Lotus Europa",
                "Ford Pantera L",
                "Ferrari Dino",
                "Maserati Bora",
                "Volvo 142E",
            ];

            // Numeric variable columns (f64 type)
            let mpg: Vec<f64> = vec![
                21.0, 21.0, 22.8, 21.4, 18.7, 18.1, 14.3, 24.4, 22.8, 19.2, 17.8, 16.4, 17.3, 15.2,
                10.4, 10.4, 14.7, 32.4, 30.4, 33.9, 21.5, 15.5, 15.2, 13.3, 19.2, 27.3, 26.0, 30.4,
                15.8, 19.7, 15.0, 21.4,
            ]; // Miles per gallon

            // Integer variable columns (i32 type)
            let cyl = vec![
                6, 6, 4, 6, 8, 6, 8, 4, 4, 6, 6, 8, 8, 8, 8, 8, 8, 4, 4, 4, 4, 8, 8, 8, 8, 4, 4, 4,
                8, 6, 8, 4,
            ]; // Number of cylinders

            let disp: Vec<f64> = vec![
                160.0, 160.0, 108.0, 258.0, 360.0, 225.0, 360.0, 146.7, 140.8, 167.6, 167.6, 275.8,
                275.8, 275.8, 472.0, 460.0, 440.0, 78.7, 75.7, 71.1, 120.1, 318.0, 304.0, 350.0,
                400.0, 79.0, 120.3, 95.1, 351.0, 145.0, 301.0, 121.0,
            ]; // Displacement (cubic inches)

            let hp = vec![
                110, 110, 93, 110, 175, 105, 245, 62, 95, 123, 123, 180, 180, 180, 205, 215, 230,
                66, 52, 65, 97, 150, 150, 245, 175, 66, 91, 113, 264, 175, 335, 109,
            ]; // Horsepower

            let drat: Vec<f64> = vec![
                3.90, 3.90, 3.85, 3.08, 3.15, 2.76, 3.21, 3.69, 3.92, 3.92, 3.92, 3.07, 3.07, 3.07,
                2.93, 3.00, 3.23, 4.08, 4.93, 4.22, 3.70, 2.76, 3.15, 3.73, 3.08, 4.08, 4.43, 3.77,
                4.22, 3.62, 3.54, 4.11,
            ]; // Rear axle ratio

            let wt: Vec<f64> = vec![
                2.620, 2.875, 2.320, 3.215, 3.440, 3.460, 3.570, 3.190, 3.150, 3.440, 3.440, 4.070,
                3.730, 3.780, 5.250, 5.424, 5.345, 2.200, 1.615, 1.835, 2.465, 3.520, 3.435, 3.840,
                3.845, 1.935, 2.140, 1.513, 3.170, 2.770, 3.570, 2.780,
            ]; // Weight (1000 lbs)

            let qsec: Vec<f64> = vec![
                16.46, 17.02, 18.61, 19.44, 17.02, 20.22, 15.84, 20.00, 22.90, 18.30, 18.90, 17.40,
                17.60, 18.00, 17.98, 17.82, 17.42, 19.47, 18.52, 19.90, 20.01, 16.87, 17.30, 15.41,
                17.05, 18.90, 16.70, 16.90, 14.50, 15.50, 14.60, 18.60,
            ]; // 1/4 mile time

            let vs = vec![
                0, 0, 1, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1,
                0, 0, 0, 1,
            ]; // Engine shape (0=V-shaped, 1=straight)

            let am = vec![
                1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1,
                1, 1, 1, 1,
            ]; // Transmission type (0=automatic, 1=manual)

            let gear = vec![
                4, 4, 4, 3, 3, 3, 3, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 4, 4, 4, 3, 3, 3, 3, 3, 4, 5, 5,
                5, 5, 5, 4,
            ]; // Number of forward gears

            let carb = vec![
                4, 4, 1, 1, 2, 1, 4, 2, 2, 4, 4, 3, 3, 3, 4, 4, 4, 1, 2, 1, 1, 2, 2, 4, 2, 1, 2, 2,
                4, 6, 8, 2,
            ]; // Number of carburetors

            // Use df! macro to build DataFrame and handle possible errors
            let df = df!(
                "model" => model,
                "mpg" => mpg,
                "cyl" => cyl,
                "disp" => disp,
                "hp" => hp,
                "drat" => drat,
                "wt" => wt,
                "qsec" => qsec,
                "vs" => vs,
                "am" => am,
                "gear" => gear,
                "carb" => carb
            )?;

            Ok(df)
        }
        // iris dataset: Classic iris flower classification dataset from https://www.kaggle.com/datasets/uciml/iris
        "iris" => {
            // Sepal length, width; Petal length, width (all numeric)
            let sepal_length: &[f64; 150] = &[
                5.1, 4.9, 4.7, 4.6, 5.0, 5.4, 4.6, 5.0, 4.4, 4.9, 5.4, 4.8, 4.8, 4.3, 5.8, 5.7,
                5.4, 5.1, 5.7, 5.1, 5.4, 5.1, 4.6, 5.1, 4.8, 5.0, 5.0, 5.2, 5.2, 4.7, 4.8, 5.4,
                5.2, 5.5, 4.9, 5.0, 5.5, 4.9, 4.4, 5.1, 5.0, 4.5, 4.4, 5.0, 5.1, 4.8, 5.1, 4.6,
                5.3, 5.0, 7.0, 6.4, 6.9, 5.5, 6.5, 5.7, 6.3, 4.9, 6.6, 5.2, 5.0, 5.9, 6.0, 6.1,
                5.6, 6.7, 5.6, 5.8, 6.2, 5.6, 5.9, 6.1, 6.3, 6.1, 6.4, 6.6, 6.8, 6.7, 6.0, 5.7,
                5.5, 5.5, 5.8, 6.0, 5.4, 6.0, 6.7, 6.3, 5.6, 5.5, 5.5, 6.1, 5.8, 5.0, 5.6, 5.7,
                5.7, 6.2, 5.1, 5.7, 6.3, 5.8, 7.1, 6.3, 6.5, 7.6, 4.9, 7.3, 6.7, 7.2, 6.5, 6.4,
                6.8, 5.7, 5.8, 6.4, 6.5, 7.7, 7.7, 6.0, 6.9, 5.6, 7.7, 6.3, 6.7, 7.2, 6.2, 6.1,
                6.4, 7.2, 7.4, 7.9, 6.4, 6.3, 6.1, 7.7, 6.3, 6.4, 6.0, 6.9, 6.7, 6.9, 5.8, 6.8,
                6.7, 6.7, 6.3, 6.5, 6.2, 5.9,
            ];

            let sepal_width: &[f64; 150] = &[
                3.5, 3.0, 3.2, 3.1, 3.6, 3.9, 3.4, 3.4, 2.9, 3.1, 3.7, 3.4, 3.0, 3.0, 4.0, 4.4,
                3.9, 3.5, 3.8, 3.8, 3.4, 3.7, 3.6, 3.3, 3.4, 3.0, 3.4, 3.5, 3.4, 3.2, 3.1, 3.4,
                4.1, 4.2, 3.1, 3.2, 3.5, 3.1, 3.0, 3.4, 3.5, 2.3, 3.2, 3.5, 3.8, 3.0, 3.8, 3.2,
                3.7, 3.3, 3.2, 3.2, 3.1, 2.3, 2.8, 2.8, 3.3, 2.4, 2.9, 2.7, 2.0, 3.0, 2.2, 2.9,
                2.9, 3.1, 3.0, 2.7, 2.2, 2.5, 3.2, 2.8, 2.5, 2.8, 2.9, 3.0, 2.8, 3.0, 2.9, 2.6,
                2.4, 2.4, 2.7, 2.7, 3.0, 3.4, 3.1, 2.3, 3.0, 2.5, 2.6, 3.0, 2.6, 2.3, 2.7, 3.0,
                2.9, 2.9, 2.5, 2.8, 3.3, 2.7, 3.0, 2.9, 3.0, 3.0, 2.5, 2.9, 2.5, 3.6, 3.2, 2.7,
                3.0, 2.5, 2.8, 3.2, 3.0, 3.8, 2.6, 2.2, 3.2, 2.8, 2.8, 2.7, 3.3, 3.2, 2.8, 3.0,
                2.8, 3.0, 2.8, 3.8, 2.8, 2.8, 2.6, 3.0, 3.4, 3.1, 3.0, 3.1, 3.1, 3.1, 2.7, 3.2,
                3.3, 3.0, 2.5, 3.0, 3.4, 3.0,
            ];

            let petal_length: &[f64; 150] = &[
                1.4, 1.4, 1.3, 1.5, 1.4, 1.7, 1.4, 1.5, 1.4, 1.5, 1.5, 1.6, 1.4, 1.1, 1.2, 1.5,
                1.3, 1.4, 1.7, 1.5, 1.7, 1.5, 1.0, 1.7, 1.9, 1.6, 1.6, 1.5, 1.4, 1.6, 1.6, 1.5,
                1.5, 1.4, 1.5, 1.2, 1.3, 1.5, 1.3, 1.5, 1.3, 1.3, 1.3, 1.6, 1.9, 1.4, 1.6, 1.4,
                1.5, 1.4, 4.7, 4.5, 4.9, 4.0, 4.6, 4.5, 4.7, 3.3, 4.6, 3.9, 3.5, 4.2, 4.0, 4.7,
                3.6, 4.4, 4.5, 4.1, 4.5, 3.9, 4.8, 4.0, 4.9, 4.7, 4.3, 4.4, 4.8, 5.0, 4.5, 3.5,
                3.8, 3.7, 3.9, 5.1, 4.5, 4.5, 4.7, 4.4, 4.1, 4.0, 4.4, 4.6, 4.0, 3.3, 4.2, 4.2,
                4.2, 4.3, 3.0, 4.1, 6.0, 5.1, 5.9, 5.6, 5.8, 6.6, 4.5, 6.3, 5.8, 6.1, 5.1, 5.3,
                5.5, 5.0, 5.1, 5.3, 5.5, 6.7, 6.9, 5.0, 5.7, 4.9, 6.7, 4.9, 5.7, 6.0, 4.8, 4.9,
                5.6, 5.8, 6.1, 6.4, 5.6, 5.1, 5.6, 6.1, 5.6, 5.5, 4.8, 5.4, 5.6, 5.1, 5.1, 5.9,
                5.7, 5.2, 5.0, 5.2, 5.4, 5.1,
            ];

            let petal_width: &[f64; 150] = &[
                0.2, 0.2, 0.2, 0.2, 0.2, 0.4, 0.3, 0.2, 0.2, 0.1, 0.2, 0.2, 0.1, 0.1, 0.2, 0.4,
                0.4, 0.3, 0.3, 0.3, 0.2, 0.4, 0.2, 0.5, 0.2, 0.2, 0.4, 0.2, 0.2, 0.2, 0.2, 0.4,
                0.1, 0.2, 0.1, 0.2, 0.2, 0.1, 0.2, 0.2, 0.3, 0.3, 0.2, 0.6, 0.4, 0.3, 0.2, 0.2,
                0.2, 0.2, 1.4, 1.5, 1.5, 1.3, 1.5, 1.3, 1.6, 1.0, 1.3, 1.4, 1.0, 1.5, 1.0, 1.4,
                1.3, 1.4, 1.5, 1.0, 1.5, 1.1, 1.8, 1.3, 1.5, 1.2, 1.3, 1.4, 1.4, 1.7, 1.5, 1.0,
                1.1, 1.0, 1.2, 1.6, 1.5, 1.6, 1.5, 1.3, 1.3, 1.3, 1.2, 1.4, 1.2, 1.0, 1.3, 1.2,
                1.3, 1.3, 1.1, 1.3, 2.5, 1.9, 2.1, 1.8, 2.2, 2.1, 1.7, 1.8, 1.8, 2.5, 2.0, 1.9,
                2.1, 2.0, 2.4, 2.3, 1.8, 2.2, 2.3, 1.5, 2.3, 2.0, 2.0, 1.8, 2.1, 1.8, 1.8, 1.8,
                2.1, 1.6, 1.9, 2.0, 2.2, 1.5, 1.4, 2.3, 2.4, 1.8, 1.8, 2.1, 2.4, 2.3, 1.9, 2.3,
                2.5, 2.3, 1.9, 2.0, 2.3, 1.8,
            ];

            // Species labels (string type), 50 records per class
            let species: Vec<&str> = (0..50)
                .map(|_| "setosa")
                .chain((0..50).map(|_| "versicolor"))
                .chain((0..50).map(|_| "virginica"))
                .collect();

            // Construct and return DataFrame
            let df = df![
                "sepal_length" => sepal_length,
                "sepal_width"  => sepal_width,
                "petal_length" => petal_length,
                "petal_width"  => petal_width,
                "species"      => species,
            ]?;

            Ok(df)
        }
        // mpg dataset: A small sample (first 10 rows) from https://www.kaggle.com/datasets/uciml/autompg-dataset
        "mpg" => {
            // Build a small DataFrame with partial fields
            let df = df![
                "mpg" =>        [18, 15, 18, 16, 17, 15, 14, 14, 14, 15],
                "cylinders" =>  [8, 8, 8, 8, 8, 8, 8, 8, 8, 8],
                "displacement" => [307, 350, 318, 304, 302, 429, 454, 440, 455, 390],
                "horsepower" => [130, 165, 150, 150, 140, 198, 220, 215, 225, 190],
                "weight" =>     [3504, 3693, 3436, 3433, 3449, 4341, 4354, 4312, 4425, 3850],
                "acceleration" => [12.0, 11.5, 11.0, 12.0, 10.5, 10.0, 9.0, 8.5, 10.0, 8.5],
                "model_year" => [70, 70, 70, 70, 70, 70, 70, 70, 70, 70],
                "origin" =>     [1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                "car_name" => [
                    "chevrolet chevelle malibu",
                    "buick skylark 320",
                    "plymouth satellite",
                    "amc rebel sst",
                    "ford torino",
                    "ford galaxie 500",
                    "chevrolet impala",
                    "plymouth fury iii",
                    "pontiac catalina",
                    "amc ambassador dpl"
                ]
            ]?;

            Ok(df)
        }
        // Nightingale wind rose dataset from https://github.com/stdlib-js/datasets-nightingales-rose/blob/main/data/data.csv
        // and https://github.com/vincentarelbundock/Rdatasets/blob/master/csv/HistData/Nightingale.csv
        "nightingale" => {
            let date: &[&str; 36] = &[
                "1854-04-01T07:00:00.000Z",
                "1854-05-01T07:00:00.000Z",
                "1854-06-01T07:00:00.000Z",
                "1854-07-01T07:00:00.000Z",
                "1854-08-01T07:00:00.000Z",
                "1854-09-01T07:00:00.000Z",
                "1854-10-01T07:00:00.000Z",
                "1854-11-01T07:00:00.000Z",
                "1854-12-01T08:00:00.000Z",
                "1855-01-01T08:00:00.000Z",
                "1855-02-01T08:00:00.000Z",
                "1855-03-01T08:00:00.000Z",
                "1854-04-01T07:00:00.000Z",
                "1854-05-01T07:00:00.000Z",
                "1854-06-01T07:00:00.000Z",
                "1854-07-01T07:00:00.000Z",
                "1854-08-01T07:00:00.000Z",
                "1854-09-01T07:00:00.000Z",
                "1854-10-01T07:00:00.000Z",
                "1854-11-01T07:00:00.000Z",
                "1854-12-01T08:00:00.000Z",
                "1855-01-01T08:00:00.000Z",
                "1855-02-01T08:00:00.000Z",
                "1855-03-01T08:00:00.000Z",
                "1854-04-01T07:00:00.000Z",
                "1854-05-01T07:00:00.000Z",
                "1854-06-01T07:00:00.000Z",
                "1854-07-01T07:00:00.000Z",
                "1854-08-01T07:00:00.000Z",
                "1854-09-01T07:00:00.000Z",
                "1854-10-01T07:00:00.000Z",
                "1854-11-01T07:00:00.000Z",
                "1854-12-01T08:00:00.000Z",
                "1855-01-01T08:00:00.000Z",
                "1855-02-01T08:00:00.000Z",
                "1855-03-01T08:00:00.000Z",
            ];

            let month: &[&str; 36] = &[
                "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
                "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
                "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
            ];

            let year: &[&str; 36] = &[
                "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1855",
                "1855", "1855", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1854",
                "1854", "1855", "1855", "1855", "1854", "1854", "1854", "1854", "1854", "1854",
                "1854", "1854", "1854", "1855", "1855", "1855",
            ];

            let army: &[u32; 36] = &[
                8571, 23333, 28333, 28722, 30246, 30290, 30643, 29736, 32779, 32393, 30919, 30107,
                8571, 23333, 28333, 28722, 30246, 30290, 30643, 29736, 32779, 32393, 30919, 30107,
                8571, 23333, 28333, 28722, 30246, 30290, 30643, 29736, 32779, 32393, 30919, 30107,
            ];

            let deaths: &[u32; 36] = &[
                0, 0, 0, 0, 1, 81, 132, 287, 114, 83, 42, 32, 5, 9, 6, 23, 30, 70, 128, 106, 131,
                324, 361, 172, 1, 12, 11, 359, 828, 788, 503, 844, 1725, 2761, 2120, 1205,
            ];

            let cause: &[&str; 36] = &[
                "Wounds", "Wounds", "Wounds", "Wounds", "Wounds", "Wounds", "Wounds", "Wounds",
                "Wounds", "Wounds", "Wounds", "Wounds", "Other", "Other", "Other", "Other",
                "Other", "Other", "Other", "Other", "Other", "Other", "Other", "Other", "Disease",
                "Disease", "Disease", "Disease", "Disease", "Disease", "Disease", "Disease",
                "Disease", "Disease", "Disease", "Disease",
            ];

            let df = df![
                "Date" => date,
                "Month" => month,
                "Year" => year,
                "Army" => army,
                "Deaths" => deaths,
                "Cause" => cause,
            ]?;

            Ok(df)
        }
        // No matching known dataset name case
        _ => Err(ChartonError::Data("Dataset not found".into())),
    }
}
