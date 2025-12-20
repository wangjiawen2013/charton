use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Sample data: Country information
    let df = df! [
        "GDP" => [21.43, 14.34, 5.31, 4.94, 3.87, 3.75], // Trillions USD
        "Population" => [331.9, 143.9, 1380.0, 279.6, 67.2, 67.0], // Millions
        "Country" => ["USA", "Russia", "China", "Brazil", "UK", "France"],
        "Continent" => ["North America", "Europe", "Asia", "South America", "Europe", "Europe"]
    ]?;

    // Create a text chart showing countries with GDP vs Population using the new API
    let text_chart = Chart::build(&df)?
        .mark_text()
        .with_text_size(16.0)
        .encode((
            x("GDP"),
            y("Population"),
            text("Country"),
            color("Continent"),
        ))?
        .swap_axes();

    // Create a layered chart and add the text chart as a layer
    LayeredChart::new()
        .with_x_label("GDP (Trillion USD)")
        .with_y_label("Population (Millions)")
        .add_layer(text_chart)
        .save("./examples/text.svg")?;

    Ok(())
}
