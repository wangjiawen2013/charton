use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create data for multiple groups (e.g., sine and cosine curves)
    let x_values: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let y_sine: Vec<f64> = x_values.iter().map(|&x| x.sin()).collect();
    let y_cosine: Vec<f64> = x_values.iter().map(|&x| x.cos()).collect();

    // Combine data into a single DataFrame with categories
    let x = x_values.repeat(2); // Repeat x values for both groups
    let y = [y_sine.clone(), y_cosine.clone()].concat();
    let category = [vec!["Sine"; 30], vec!["Cosine"; 30]].concat();

    // Create a line chart with multiple groups
    chart!(x, y, category)?
        .mark_line()?
        .configure_line(|l| l.with_loess(true).with_loess_bandwidth(0.2))
        .encode((
            alt::x("x"),
            alt::y("y"),
            alt::color("category"), // This creates separate lines for each category
        ))?
        .configure_theme(|t| t.with_x_tick_label_angle(-45.0))
        .coord_flip()
        .save("docs/src/images/line.svg")?;

    Ok(())
}
