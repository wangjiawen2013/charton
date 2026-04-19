use crate::core::data::Dataset;
use crate::error::ChartonError;

// Nightingale wind rose dataset from https://github.com/stdlib-js/datasets-nightingales-rose/blob/main/data/data.csv
// and https://github.com/vincentarelbundock/Rdatasets/blob/master/csv/HistData/Nightingale.csv
pub fn get_data() -> Result<Dataset, ChartonError> {
    let date: Vec<&str> = vec![
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

    let month: Vec<&str> = vec![
        "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar", "Apr",
        "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar", "Apr", "May",
        "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
    ];

    let year: Vec<&str> = vec![
        "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1855", "1855",
        "1855", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1855",
        "1855", "1855", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1854", "1854",
        "1855", "1855", "1855",
    ];

    let army: Vec<u32> = vec![
        8571, 23333, 28333, 28722, 30246, 30290, 30643, 29736, 32779, 32393, 30919, 30107, 8571,
        23333, 28333, 28722, 30246, 30290, 30643, 29736, 32779, 32393, 30919, 30107, 8571, 23333,
        28333, 28722, 30246, 30290, 30643, 29736, 32779, 32393, 30919, 30107,
    ];

    let deaths: Vec<u32> = vec![
        0, 0, 0, 0, 1, 81, 132, 287, 114, 83, 42, 32, 5, 9, 6, 23, 30, 70, 128, 106, 131, 324, 361,
        172, 1, 12, 11, 359, 828, 788, 503, 844, 1725, 2761, 2120, 1205,
    ];

    let cause: Vec<&str> = vec![
        "Wounds", "Wounds", "Wounds", "Wounds", "Wounds", "Wounds", "Wounds", "Wounds", "Wounds",
        "Wounds", "Wounds", "Wounds", "Other", "Other", "Other", "Other", "Other", "Other",
        "Other", "Other", "Other", "Other", "Other", "Other", "Disease", "Disease", "Disease",
        "Disease", "Disease", "Disease", "Disease", "Disease", "Disease", "Disease", "Disease",
        "Disease",
    ];

    let ds = Dataset::new()
        .with_column("Date", date)?
        .with_column("Month", month)?
        .with_column("Year", year)?
        .with_column("Army", army)?
        .with_column("Deaths", deaths)?
        .with_column("Cause", cause)?;

    Ok(ds)
}
