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
    Chart::build(&df)?
        .mark_text()?
        .configure_text(|t| t.with_size(12.0))
        .encode((
            x("GDP"),
            y("Population"),
            text("Country"),
            color("Continent"),
        ))?
        .with_x_label("GDP (Trillion USD)")
        .with_y_label("Population (Millions)")
        .with_x_expand(Expansion {
            mult: (0.1, 0.1),
            add: (0.1, 0.1),
        })
        .save("docs/src/images/text.svg")?;

    Ok(())
}
