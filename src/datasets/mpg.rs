use crate::error::ChartonError;
use polars::prelude::*;

// mpg dataset: A small sample (first 10 rows) from https://www.kaggle.com/datasets/uciml/autompg-dataset
pub fn get_data() -> Result<DataFrame, ChartonError> {
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
