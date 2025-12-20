use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_boxplot_1() -> Result<(), Box<dyn Error>> {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some("./datasets/iris.csv".into()))?
        .finish()?;
    let df_melted = df.unpivot(
        ["sepal length", "sepal width", "petal length", "petal width"],
        ["class"],
    )?;
    println!("{}", &df_melted);

    Chart::build(&df_melted)?
        .mark_boxplot()
        .encode((x("variable"), y("value"), color("class")))?
        .swap_axes()
        .into_layered()
        .save("./tests/boxplot_1.svg")?;

    Ok(())
}
