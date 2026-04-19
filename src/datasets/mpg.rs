use crate::core::data::{Dataset, IntoColumn, ToDataset};
use crate::error::ChartonError;

// mpg dataset: A small sample (first 10 rows) from https://www.kaggle.com/datasets/uciml/autompg-dataset
pub fn get_data() -> Result<Dataset, ChartonError> {
    // Build a small DataFrame with partial fields
    let raw_data = vec![
        (
            "mpg",
            vec![18, 15, 18, 16, 17, 15, 14, 14, 14, 15].into_column(),
        ),
        (
            "cylinders",
            vec![8, 8, 8, 8, 8, 8, 8, 8, 8, 8].into_column(),
        ),
        (
            "displacement",
            vec![307, 350, 318, 304, 302, 429, 454, 440, 455, 390].into_column(),
        ),
        (
            "horsepower",
            vec![130, 165, 150, 150, 140, 198, 220, 215, 225, 190].into_column(),
        ),
        (
            "weight",
            vec![3504, 3693, 3436, 3433, 3449, 4341, 4354, 4312, 4425, 3850].into_column(),
        ),
        (
            "acceleration",
            vec![12.0, 11.5, 11.0, 12.0, 10.5, 10.0, 9.0, 8.5, 10.0, 8.5].into_column(),
        ),
        (
            "model_year",
            vec![70, 70, 70, 70, 70, 70, 70, 70, 70, 70].into_column(),
        ),
        ("origin", vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1].into_column()),
        (
            "car_name",
            vec![
                "chevrolet chevelle malibu",
                "buick skylark 320",
                "plymouth satellite",
                "amc rebel sst",
                "ford torino",
                "ford galaxie 500",
                "chevrolet impala",
                "plymouth fury iii",
                "pontiac catalina",
                "amc ambassador dpl",
            ]
            .into_column(),
        ),
    ];

    let ds = raw_data.to_dataset()?;

    Ok(ds)
}
