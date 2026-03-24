use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_boxplot_1() -> Result<(), Box<dyn Error>> {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some("./assets/iris.csv".into()))?
        .finish()?;
    let df_melted = df.unpivot(
        ["sepal_length", "sepal_width", "petal_length", "petal_width"],
        ["species"],
    )?;
    println!("{}", &df_melted);

    Chart::build(&df_melted)?
        .mark_boxplot()?
        .encode((x("variable"), y("value"), color("species")))?
        .coord_flip()
        .save("./tests/boxplot_1.svg")?;

    Ok(())
}
