use crate::core::data::Dataset;
use crate::error::ChartonError;
use time::{Date, Month, OffsetDateTime};

// unemployment dataset: A sub set of country's unemployment rate from past 31 years from https://www.kaggle.com/datasets/pantanjali/unemployment-dataset
pub fn get_data() -> Result<Dataset, ChartonError> {
    let country = vec![
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
        "Belarus",
        "Ireland",
        "Moldova",
        "Malta",
        "Puerto Rico",
        "Senegal",
        "Slovenia",
    ];
    let year = vec![
        2011, 2011, 2011, 2011, 2011, 2011, 2011, 2012, 2012, 2012, 2012, 2012, 2012, 2012, 2013,
        2013, 2013, 2013, 2013, 2013, 2013, 2014, 2014, 2014, 2014, 2014, 2014, 2014, 2015, 2015,
        2015, 2015, 2015, 2015, 2015, 2016, 2016, 2016, 2016, 2016, 2016, 2016, 2017, 2017, 2017,
        2017, 2017, 2017, 2017, 2018, 2018, 2018, 2018, 2018, 2018, 2018, 2019, 2019, 2019, 2019,
        2019, 2019, 2019, 2020, 2020, 2020, 2020, 2020, 2020, 2020, 2021, 2021, 2021, 2021, 2021,
        2021, 2021,
    ];
    let unemployment = vec![
        6.17, 15.35, 6.68, 6.38, 15.7, 10.36, 8.17, 6.05, 15.45, 5.58, 6.2, 14.5, 9.44, 8.84, 6.01,
        13.73, 5.1, 6.11, 14.3, 8.58, 10.1, 5.99, 11.86, 3.73, 5.72, 13.9, 7.65, 9.67, 5.84, 9.91,
        4.7, 5.38, 12.0, 6.76, 8.96, 5.84, 8.37, 4.02, 4.69, 11.8, 4.46, 8.0, 5.65, 6.71, 4.1, 4.0,
        10.8, 3.69, 6.56, 4.76, 5.74, 4.11, 3.66, 9.2, 3.28, 5.11, 4.16, 4.95, 5.1, 3.62, 8.3,
        2.86, 4.45, 4.77, 5.62, 3.82, 4.26, 8.89, 3.62, 4.97, 4.74, 6.63, 3.96, 3.5, 8.27, 3.72,
        4.42,
    ];

    // Convert i32 years into OffsetDateTime
    let year_dates: Vec<OffsetDateTime> = year
        .iter()
        .map(|&y| {
            Date::from_calendar_date(y, Month::January, 1)
                // 1. Create a "Date" object (Year-Month-Day).
                // It only contains the calendar day, no time information yet.
                .expect("invalid date")
                // 2. Transform "Date" into "PrimitiveDateTime".
                // Adds a time component (00:00:00.000) to the specific date.
                .midnight()
                // 3. Transform "PrimitiveDateTime" into "OffsetDateTime".
                // Attaches a TimeZone (UTC offset +00:00) so it becomes a
                // fixed point in global history (a "Timestamp").
                .assume_utc()
        })
        .collect();

    // Construct the Dataset
    let ds = Dataset::new()
        .with_column("Country", country)?
        .with_column("Year", year_dates)?
        .with_column("Unemployment rate (%)", unemployment)?;

    Ok(ds)
}
