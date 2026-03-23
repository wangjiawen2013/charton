pub mod iris;
pub mod mpg;
pub mod mtcars;
pub mod nightingale;
pub mod penguins;
pub mod unemployment;

use crate::error::ChartonError;
use polars::prelude::*;

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
///   - `"penguins"`: Detaset that includes data points across a sample size of 344 penguins (344 rows × 7 columns).
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
        "mtcars" => mtcars::get_data(),
        "iris" => iris::get_data(),
        "mpg" => mpg::get_data(),
        "penguins" => penguins::get_data(),
        "nightingale" => nightingale::get_data(),
        "unemployment" => unemployment::get_data(),
        _ => Err(ChartonError::Data("Dataset not found".into())),
    }
}
