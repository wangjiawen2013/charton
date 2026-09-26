//! Production-style label formatting.
//!
//! Reproduces the scenario from the issue: an "Economist-style" investment
//! chart where the y axis shows large values as `$20B` instead of raw numbers,
//! and the legend labels are tidied up.
//!
//! Run with: `cargo run --example label_format`

use charton::core::guide::LegendPosition;
use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Three focus areas, eight years, values in dollars.
    let years: Vec<f64> = (2015..2023).map(|year| year as f64).collect();
    let areas = [
        "Medical and healthcare",
        "Data management, processing, cloud",
        "Retail",
    ];

    let mut x = Vec::new();
    let mut investment = Vec::new();
    let mut area = Vec::new();
    for (index, name) in areas.iter().enumerate() {
        for year in &years {
            x.push(*year);
            // A made-up exponential-ish growth curve per area.
            let base = 2.0e9 * (index as f64 + 1.0);
            investment.push(base * (1.0 + (year - 2015.0) * 0.45).powf(1.8));
            area.push(*name);
        }
    }

    // Dollars, abbreviated, no decimals: 20_000_000_000 -> "$20B".
    let money = LabelFormat::new()
        .with_prefix("$")
        .with_compact_notation()
        .with_precision(0);

    // Legend labels: turn the long data names into short, readable ones.
    let short_names = LabelFormat::new().with_text_formatter(|label| match label {
        "Medical and healthcare" => "Healthcare".to_string(),
        "Data management, processing, cloud" => "Data & cloud".to_string(),
        other => other.to_string(),
    });

    chart!(x, investment, area)?
        .mark_line()?
        .encode((alt::x("x"), alt::y("investment"), alt::color("area")))?
        .with_size(760, 460)
        .with_x_label("Year")
        .with_y_label("")
        .with_color_label("Focus area")
        .with_y_label_format(money)
        .with_legend_label_format(short_names)
        .configure_theme(|theme| theme.with_legend_position(LegendPosition::Top))
        .save("docs/src/images/label_format.svg")?;

    Ok(())
}
