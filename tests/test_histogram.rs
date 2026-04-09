use charton::prelude::*;
use std::error::Error;

#[test]
fn test_histogram_1() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("iris")?;
    println!("{}", &ds);

    // Create a histogram chart
    let histogram = chart!(ds)?
        .mark_hist()?
        .configure_hist(|h| {
            h.with_color("steelblue")
                .with_opacity(0.5)
                .with_stroke("black")
                .with_stroke_width(0.0)
        })
        .encode((
            alt::x("sepal_length"),
            alt::y("count").with_normalize(true),
            alt::color("species"),
        ))?;

    histogram
        .with_size(600, 400)
        .with_title("Histogram Example")
        .with_x_label("Value")
        .with_y_label("Frequency")
        .configure_theme(|t| t.with_palette(ColorPalette::Tab10))
        .coord_flip()
        .save("./tests/histogram_1.svg")?;

    Ok(())
}
