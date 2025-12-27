use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_histogram_1() -> Result<(), Box<dyn Error>> {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some("./datasets/iris.csv".into()))?
        .finish()?;
    let df_melted = df.unpivot(
        ["sepal length", "sepal width", "petal length", "petal width"],
        ["class"],
    )?;
    println!("{}", &df_melted);

    // Create a histogram chart
    let histogram_chart = Chart::build(&df_melted.head(Some(200)))?
        .mark_hist()
        .encode((
            x("value"),
            y("count").with_normalize(true),
            color("variable"),
        ))?
        .with_hist_color(Some(SingleColor::new("steelblue")))
        .with_hist_opacity(0.5)
        .with_hist_stroke(Some(SingleColor::new("black")))
        .with_hist_stroke_width(0.0)
        .with_color_palette(ColorPalette::Tab10);

    // Create a layered chart for the histogram
    LayeredChart::new()
        .with_size(600, 400)
        .with_title("Histogram Example")
        .with_x_label("Value")
        .with_y_label("Frequency")
        .add_layer(histogram_chart)
        .with_legend(true)
        .swap_axes()
        .save("./tests/histogram_1.svg")?;

    Ok(())
}
