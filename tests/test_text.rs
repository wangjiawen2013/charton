use charton::prelude::*;
use std::error::Error;

#[test]
fn test_text_1() -> Result<(), Box<dyn Error>> {
    // Sample data: Country information
    let gdp = [21.43, 14.34, 5.31, 4.94, 3.87, 3.75]; // Trillions USD
    let population = [331.9, 143.9, 1380.0, 279.6, 67.2, 67.0]; // Millions
    let country = ["USA", "Russia", "China", "Brazil", "UK", "France"];
    let continent = [
        "North America",
        "Europe",
        "Asia",
        "South America",
        "Europe",
        "Europe",
    ];

    // Create a text chart showing countries with GDP vs Population using the new API
    chart!(gdp, population, country, continent)?
        .mark_text()?
        .configure_text(|t| t.with_size(16.0))
        .encode((
            alt::x("gdp"),
            alt::y("population"),
            alt::text("country"),
            alt::color("continent"),
        ))?
        .with_size(600, 400)
        .with_x_label("GDP (Trillion USD)")
        .with_y_label("Population (Millions)")
        .coord_flip()
        .save("./tests/text_1.svg")?;

    Ok(())
}
