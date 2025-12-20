use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn tests_transform_window_1() -> Result<(), Box<dyn Error>> {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some("./datasets/iris.csv".into()))?
        .finish()?;

    // Create a chart with window transform
    let chart = Chart::build(&df.select(["class", "sepal length"])?)?
        .transform_window(
            WindowTransform::new(WindowFieldDef::new(
                "sepal length",
                WindowOnlyOp::CumeDist,
                "ecdf", // This will be the output column name
            ))
            .with_groupby("class")
            .with_normalize(false), // Normalize to [0,1] range
        )?
        .mark_line()
        .encode((x("sepal length"), y("ecdf"), color("class")))?;

    // Create layered chart for display
    LayeredChart::new()
        .with_size(600, 400)
        .with_title("Empirical Cumulative Distribution")
        .add_layer(chart)
        .save("./tests/transform_window_1.svg")?;

    Ok(())
}
