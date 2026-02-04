use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some("./datasets/iris.csv".into()))?
        .finish()?;
    let df_melted = df.unpivot(
        ["sepal_length", "sepal_width", "petal_length", "petal_width"],
        ["species"],
    )?;
    println!("{}", &df_melted);

    // Create a histogram chart
    let histogram_chart = Chart::build(&df_melted.head(Some(200)))?
        .mark_hist()
        .configure_hist(|h| h.with_color("steelblue")
            .with_opacity(0.5)
            .with_stroke("black")
            .with_stroke_width(0.0)
        )
        .encode((
            x("value"),
            y("count").with_normalize(true),
            color("variable"),
        ))?;

    // Create a layered chart for the histogram
    LayeredChart::new()
        .with_size(500, 400)
        .with_title("Histogram Example")
        .with_x_label("Value")
        .with_y_label("Frequency")
        .add_layer(histogram_chart)
        .configure_theme(|t| t.with_palette(ColorPalette::Tab10))
        .save("./examples/histogram.svg")?;

    Ok(())
}