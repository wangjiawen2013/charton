use charton::prelude::*;
use std::error::Error;

#[test]
fn test_tick_1() -> Result<(), Box<dyn Error>> {
    // Create a sample DataFrame with precipitation data by category
    // Similar to Altair's seattle_weather dataset
    let category = ["A", "B", "A", "C", "B", "A", "C", "B", "A", "C"];
    let precipitation = [0.1, 0.5, 0.2, 0.8, 0.3, 0.15, 0.6, 0.4, 0.25, 0.7];

    // Shows distribution of precipitation within each category
    let chart = chart!(category, precipitation)?
        .mark_tick()?
        .encode((
            alt::x("precipitation"),
            alt::y("category"),
            alt::color("category"),
        ))?
        .configure_tick(|m| {
            m.with_thickness(2.0)
                .with_band_size(10.0)
                .with_color("blue")
        });
    chart.save("./tests/test_tick_1.svg")?;

    Ok(())
}

#[test]
fn test_tick_2() -> Result<(), Box<dyn Error>> {
    // Create a sample DataFrame with precipitation data by category
    // Similar to Altair's seattle_weather dataset
    let category = [0.1, 0.2, 0.1, 0.3, 0.2, 0.1, 0.3, 0.2, 0.1, 0.3];
    let precipitation = [0.1, 0.5, 0.2, 0.8, 0.3, 0.15, 0.6, 0.4, 0.25, 0.7];

    // Shows distribution of precipitation within each category
    let chart = chart!(category, precipitation)?
        .mark_tick()?
        .encode((alt::x("precipitation"), alt::y("category")))?
        .coord_flip();
    chart.save("./tests/test_tick_2.svg")?;

    Ok(())
}
