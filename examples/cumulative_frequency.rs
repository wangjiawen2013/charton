use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some("./datasets/iris.csv".into()))?
        .finish()?;
    //let df_melted = df.unpivot(["sepal length", "sepal width", "petal length", "petal width"], ["class"])?;
    //println!("{}", &df_melted);

    // Create a chart with window transform
    let chart = Chart::build(&df.select(["class", "sepal length"])?)?
        .transform_window(
            WindowTransform::new(WindowFieldDef::new(
                "sepal length",
                WindowOnlyOp::CumeDist,
                "ecdf", // This will be the output column name
            ))
            .with_groupby("class")
            .with_normalize(false),
        )?
        .mark_line()
        .with_interpolation(PathInterpolation::StepAfter) // Add step interpolation
        .encode((x("sepal length"), y("ecdf"), color("class")))?;

    // Create layered chart for display
    LayeredChart::new()
        .with_title("Empirical Cumulative Distribution")
        .add_layer(chart)
        .save("./examples/cumulative_frequency.svg")?;

    Ok(())
}
