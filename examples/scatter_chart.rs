use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = load_dataset("mtcars")?
        .lazy()
        .with_columns([col("gear").cast(DataType::String)])
        .collect()?;

    Chart::build(&df)?
        .mark_point()
        .encode((x("wt"), y("mpg"), color("gear")))?
        .into_layered()
        .coord_flip()
        .with_x_tick_label_angle(-45.0)
        .save("./examples/scatter_chart.svg")?;

    Ok(())
}