use crate::coord::Scale;
use crate::error::ChartonError;
use polars::prelude::*;
use std::collections::HashMap;
use std::vec::Vec;

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
/// ```
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

// Implementation for converting &DataFrame to DataFrameSource
impl TryFrom<&DataFrame> for DataFrameSource {
    type Error = ChartonError;

    /// Creates a new DataFrameSource from a reference to a DataFrame.
    ///
    /// This creates a clone of the DataFrame when Polars versions match,
    /// or suggests using Parquet serialization for version compatibility.
    ///
    /// # Arguments
    /// * `df` - A reference to the DataFrame to wrap in a DataFrameSource
    ///
    /// # Returns
    /// A new DataFrameSource instance containing a clone of the provided DataFrame.
    ///
    /// # Note
    /// For cross-version compatibility, serialize your DataFrame to Parquet format
    /// and use `DataFrameSource::try_from(Vec<u8>)` instead.
    fn try_from(df: &DataFrame) -> Result<Self, Self::Error> {
        Ok(DataFrameSource::new(df.clone()))
    }
}

// Implementation for converting &LazyFrame to DataFrameSource
impl TryFrom<&LazyFrame> for DataFrameSource {
    type Error = ChartonError;

    /// Creates a new DataFrameSource from a reference to a LazyFrame.
    ///
    /// This collects the LazyFrame into a DataFrame when Polars versions match,
    /// or suggests using Parquet serialization for version compatibility.
    ///
    /// # Arguments
    /// * `lf` - A reference to the LazyFrame to collect and wrap
    ///
    /// # Returns
    /// A new DataFrameSource instance containing the collected DataFrame
    ///
    /// # Note
    /// For cross-version compatibility with Polars, first collect your LazyFrame
    /// into a DataFrame and then serialize it to Parquet format before creating
    /// a DataFrameSource.
    ///
    /// # Errors
    /// Returns a ChartonError if the LazyFrame fails to collect into a DataFrame.
    fn try_from(lf: &LazyFrame) -> Result<Self, Self::Error> {
        let df = lf.clone().collect()?;
        Ok(DataFrameSource::new(df))
    }
}

// Implementation for converting Vec<u8> (Parquet data) to DataFrameSource
impl TryFrom<&Vec<u8>> for DataFrameSource {
    type Error = ChartonError;

    /// Creates a new DataFrameSource from Parquet data.
    ///
    /// This allows users to pass DataFrame serialized as Parquet data,
    /// enabling interoperability between different Polars versions.
    ///
    /// # Arguments
    /// * `parquet_data` - A reference to the vector of bytes containing Parquet-serialized DataFrame
    ///
    /// # Returns
    /// A new DataFrameSource instance containing the deserialized DataFrame
    ///
    /// # Errors
    /// Returns a ChartonError if the Parquet data cannot be read into a DataFrame.
    fn try_from(parquet_data: &Vec<u8>) -> Result<Self, Self::Error> {
        let cursor = std::io::Cursor::new(parquet_data);
        let df = ParquetReader::new(cursor).finish()?;
        Ok(DataFrameSource::new(df))
    }
}

// Determine the appropriate default scale type based on the data type
// (Linear) or discrete scale mapping
pub(crate) fn determine_scale_for_dtype(dtype: &polars::datatypes::DataType) -> Scale {
    use polars::datatypes::DataType::*;

    match dtype {
        // Continuous numeric types
        UInt8 | UInt16 | UInt32 | UInt64 | Int8 | Int16 | Int32 | Int64 | Int128 | Float32
        | Float64 => Scale::Linear,

        // All other types default to discrete
        _ => Scale::Discrete,
    }
}

// Checks if a DataFrame contains the required columns, and optionally verifies the data types
//
// # Arguments
// * `df` - The DataFrame to check
// * `required_columns` - A slice of column names that must exist
// * `expected_types` - Map of column names to expected DataTypes; only columns in the map are checked
//
// # Returns
// * `Ok(())` if all required columns exist and match one of the expected types (if provided)
// * `Err(String)` if a column is missing or a type does not match any of the expected types
pub(crate) fn check_schema(
    df: &mut DataFrame,
    required_columns: &[&str],
    expected_types: &HashMap<&str, Vec<DataType>>,
) -> Result<(), ChartonError> {
    let schema = df.schema();

    for &col_name in required_columns {
        let actual_type = schema.get(col_name).ok_or_else(|| {
            ChartonError::Encoding(format!("Column '{}' not found in DataFrame", col_name))
        })?;

        // Check type if it's specified in the expected_types map
        if let Some(expected_types_vec) = expected_types.get(col_name)
            && !expected_types_vec.contains(actual_type) {
                return Err(ChartonError::Data(format!(
                    "Column '{}' has type {:?}, but expected one of {:?}",
                    col_name, actual_type, expected_types_vec
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
///
/// # Returns
///
/// On success, returns a `DataFrame` containing the specified dataset content;
/// on failure, returns a `ChartonError`.
///
/// # Examples
///
/// ```
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
            let mpg = vec![
                21.0, 21.0, 22.8, 21.4, 18.7, 18.1, 14.3, 24.4, 22.8, 19.2, 17.8, 16.4, 17.3, 15.2,
                10.4, 10.4, 14.7, 32.4, 30.4, 33.9, 21.5, 15.5, 15.2, 13.3, 19.2, 27.3, 26.0, 30.4,
                15.8, 19.7, 15.0, 21.4,
            ]; // Miles per gallon

            // Integer variable columns (i32 type)
            let cyl = vec![
                6, 6, 4, 6, 8, 6, 8, 4, 4, 6, 6, 8, 8, 8, 8, 8, 8, 4, 4, 4, 4, 8, 8, 8, 8, 4, 4, 4,
                8, 6, 8, 4,
            ]; // Number of cylinders

            let disp = vec![
                160.0, 160.0, 108.0, 258.0, 360.0, 225.0, 360.0, 146.7, 140.8, 167.6, 167.6, 275.8,
                275.8, 275.8, 472.0, 460.0, 440.0, 78.7, 75.7, 71.1, 120.1, 318.0, 304.0, 350.0,
                400.0, 79.0, 120.3, 95.1, 351.0, 145.0, 301.0, 121.0,
            ]; // Displacement (cubic inches)

            let hp = vec![
                110, 110, 93, 110, 175, 105, 245, 62, 95, 123, 123, 180, 180, 180, 205, 215, 230,
                66, 52, 65, 97, 150, 150, 245, 175, 66, 91, 113, 264, 175, 335, 109,
            ]; // Horsepower

            let drat = vec![
                3.90, 3.90, 3.85, 3.08, 3.15, 2.76, 3.21, 3.69, 3.92, 3.92, 3.92, 3.07, 3.07, 3.07,
                2.93, 3.00, 3.23, 4.08, 4.93, 4.22, 3.70, 2.76, 3.15, 3.73, 3.08, 4.08, 4.43, 3.77,
                4.22, 3.62, 3.54, 4.11,
            ]; // Rear axle ratio

            let wt = vec![
                2.620, 2.875, 2.320, 3.215, 3.440, 3.460, 3.570, 3.190, 3.150, 3.440, 3.440, 4.070,
                3.730, 3.780, 5.250, 5.424, 5.345, 2.200, 1.615, 1.835, 2.465, 3.520, 3.435, 3.840,
                3.845, 1.935, 2.140, 1.513, 3.170, 2.770, 3.570, 2.780,
            ]; // Weight (1000 lbs)

            let qsec = vec![
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
            let sepal_length = &[
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

            let sepal_width = &[
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

            let petal_length = &[
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

            let petal_width = &[
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
        // No matching known dataset name case
        _ => Err(ChartonError::Data("Dataset not found".into())),
    }
}
