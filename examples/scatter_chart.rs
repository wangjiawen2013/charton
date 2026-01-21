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
        //.configure_point(|m| m.color("red").shape("star").size(3.0).opacity(0.5))
        .encode((x("wt"), y("mpg")))?
        .encode((x("wt"), y("mpg"), color("gear"), shape("gear"), size("mpg")))?
        .into_layered()
        .coord_flip()
        .configure_theme(|t| t.x_tick_label_angle(-45.0))
        .title("abc")
        .save("./examples/scatter_chart.svg")?;

    Ok(())
}