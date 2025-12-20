use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create data for multiple groups (e.g., sine and cosine curves)
    let x_values: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let y_sine: Vec<f64> = x_values.iter().map(|&x| x.sin()).collect();
    let y_cosine: Vec<f64> = x_values.iter().map(|&x| x.cos()).collect();

    // Combine data into a single DataFrame with categories
    let df = df![
        "x" => x_values.repeat(2),  // Repeat x values for both groups
        "y" => [y_sine.clone(), y_cosine.clone()].concat(),
        "category" => [
            vec!["Sine"; 30],
            vec!["Cosine"; 30]
        ].concat()
    ]?;

    // Create a line chart with multiple groups
    let chart = Chart::build(&df)?.mark_line().encode((
        x("x"),
        y("y"),
        color("category"), // This creates separate lines for each category
    ))?;

    LayeredChart::new()
        .add_layer(chart)
        .with_x_tick_label_angle(45.0)
        .save("./examples/line.svg")?;

    Ok(())
}
