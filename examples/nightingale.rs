use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Creating a high-contrast dataset for Emergency Response Costs.
    // The "Normal" months (Apr, May, Oct) are kept intentionally tiny.
    // The "Extreme" months (Jan, Jul, Dec) are given massive values.
    let df = df! [
        "Month" => [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun",
            "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
        ].repeat(3),
        
        "Category" => [
            // Layer 1: Lighting (The core - relatively stable)
            "Lighting", "Lighting", "Lighting", "Lighting", "Lighting", "Lighting",
            "Lighting", "Lighting", "Lighting", "Lighting", "Lighting", "Lighting",
            // Layer 2: Appliances (The middle - baseline needs)
            "Appliances", "Appliances", "Appliances", "Appliances", "Appliances", "Appliances",
            "Appliances", "Appliances", "Appliances", "Appliances", "Appliances", "Appliances",
            // Layer 3: Heating/Extreme (The outer petal - the source of drama)
            "Heating", "Heating", "Heating", "Heating", "Heating", "Heating",
            "Heating", "Heating", "Heating", "Heating", "Heating", "Heating"
        ],

        "Usage_KWh" => [
            // Lighting: Subtle variation
            120.0, 100.0, 60.0, 30.0, 20.0, 10.0, 5.0, 25.0, 40.0, 70.0, 110.0, 130.0,
            
            // Appliances: Consistent baseline
            180.0, 120.0, 140.0, 150.0, 60.0, 80.0, 60.0, 110.0, 150.0, 180.0, 190.0, 220.0,
            
            // Heating: EXTREME RADIUS DIFFERENTIATION
            // January and December are 150x larger than June!
            1500.0, 800.0, 150.0, 20.0, 10.0, 5.0, 0.0, 5.0, 15.0, 100.0, 600.0, 1800.0
        ],
    ]?;

    // 2. Build the chart layer. 
    let energy_layer = Chart::build(&df)?
        .mark_bar()?
        .encode((
            x("Month"),
            y("Usage_KWh").with_stack(true).with_normalize(false),
            color("Category"),
        ))?;

    // 3. Final Assembly in Polar Coordinates.
    LayeredChart::new()
        .with_title("Extreme Seasonal Consumption Variance")
        .add_layer(energy_layer)
        .with_coord(CoordSystem::Polar)
        .save("./examples/nightingale.svg")?;

    Ok(())
}