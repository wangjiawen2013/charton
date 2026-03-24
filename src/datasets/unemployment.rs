use crate::error::ChartonError;
use polars::prelude::*;
//use time::OffsetDateTime;

// unemployment dataset: A sub set of country's unemployment rate from past 31 years from https://www.kaggle.com/datasets/pantanjali/unemployment-dataset
pub fn get_data() -> Result<DataFrame, ChartonError> {
    let country = [
        "Afghanistan",
        "Burundi",
        "Canada",
        "Switzerland",
        "Chile",
        "China",
        "Cameroon",
        "Afghanistan",
        "Burundi",
        "Canada",
        "Switzerland",
        "Chile",
        "China",
        "Cameroon",
        "Afghanistan",
        "Burundi",
        "Canada",
        "Switzerland",
        "Chile",
        "China",
        "Cameroon",
        "Afghanistan",
        "Burundi",
        "Canada",
        "Switzerland",
        "Chile",
        "China",
        "Cameroon",
        "Afghanistan",
        "Burundi",
        "Canada",
        "Switzerland",
        "Chile",
        "China",
        "Cameroon",
        "Afghanistan",
        "Burundi",
        "Canada",
        "Switzerland",
        "Chile",
        "China",
        "Cameroon",
        "Afghanistan",
        "Burundi",
        "Canada",
        "Switzerland",
        "Chile",
        "China",
        "Cameroon",
        "Afghanistan",
        "Burundi",
        "Canada",
        "Switzerland",
        "Chile",
        "China",
        "Cameroon",
    ];
    let year = [
        2014, 2014, 2014, 2014, 2014, 2014, 2014, 2015, 2015, 2015, 2015, 2015, 2015, 2015, 2016,
        2016, 2016, 2016, 2016, 2016, 2016, 2017, 2017, 2017, 2017, 2017, 2017, 2017, 2018, 2018,
        2018, 2018, 2018, 2018, 2018, 2019, 2019, 2019, 2019, 2019, 2019, 2019, 2020, 2020, 2020,
        2020, 2020, 2020, 2020, 2021, 2021, 2021, 2021, 2021, 2021, 2021,
    ];
    let unemployment = [
        11.14, 1.57, 6.91, 4.83, 6.66, 4.61, 3.53, 11.13, 1.6, 6.91, 4.8, 6.51, 4.63, 3.55, 11.16,
        1.59, 7.0, 4.92, 6.74, 4.53, 3.58, 11.18, 1.59, 6.34, 4.8, 6.96, 4.44, 3.60, 11.15, 1.59,
        5.83, 4.71, 7.23, 4.28, 3.62, 11.22, 1.59, 5.66, 4.39, 7.29, 4.52, 3.64, 11.71, 1.71, 9.46,
        4.82, 11.18, 5.00, 3.84, 13.28, 1.79, 7.51, 5.32, 9.13, 4.82, 3.87,
    ];

    // Convert year to datetime in the future
    // let date_values: Vec<i64> = year_raw
    //     .iter()
    //     .map(|&year| {
    //         let dt = OffsetDateTime::new_utc(
    //             time::Date::from_calendar_date(year, time::Month::January, 1).unwrap(),
    //             time::Time::MIDNIGHT,
    //         );
    //         dt.unix_timestamp_nanos() as i64
    //     })
    //     .collect();

    // let year_series = Series::new("Year".into(), date_values)
    //     .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))?;

    let df = df![
     "Country" => country,
     "Year" => year,
     "Unemployment rate (%)" => unemployment
    ]?;

    Ok(df)
}
