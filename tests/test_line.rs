use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_line_1() -> Result<(), Box<dyn Error>> {
    // Create sample data frame with a, b, and category columns
    let df = df![
        "a" => [1.0, 2.0, 3.0, 4.0, 5.0],
        "b" => [10.0, 20.0, 30.0, 40.0, 50.0],
        "category" => ["A", "B", "A", "B", "C"]
    ]?;

    // Create a point chart with only a, b, and color encodings
    let chart = Chart::build(&df)?.mark_line().encode((
        x("a"),
        y("b"),
        //color("category"),
    ))?;

    LayeredChart::new()
        .with_size(500, 300)
        .add_layer(chart)
        .save("./tests/line_1.svg")?;

    Ok(())
}

#[test]
fn test_line_2() -> Result<(), Box<dyn Error>> {
    // Create 30 points along a sine curve
    let x_values: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let y_values: Vec<f64> = x_values.iter().map(|&x| x.sin()).collect();

    // Create DataFrame with the sine wave data
    let df = df![
        "a" => x_values,
        "b" => y_values,
        "category" => ["Sine"; 30]
    ]?;

    // Create a line chart with LOESS smoothing
    let chart = Chart::build(&df)?
        .mark_line()
        .transform_loess(0.3) // Apply LOESS smoothing with bandwidth 0.3
        .encode((x("a"), y("b"), color("category")))?;

    LayeredChart::new()
        .with_size(600, 400)
        .add_layer(chart)
        .save("./tests/line_2.svg")?;

    Ok(())
}

#[test]
fn test_line_3() -> Result<(), Box<dyn Error>> {
    // Create data for multiple groups (e.g., sine and cosine curves)
    let x_values: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let y_sine: Vec<f64> = x_values.iter().map(|&x| x.sin()).collect();
    let y_cosine: Vec<f64> = x_values.iter().map(|&x| x.cos()).collect();

    // Combine data into a single DataFrame with categories
    let df = df![
        "a" => x_values.repeat(2),  // Repeat x values for both groups
        "b" => [y_sine, y_cosine].concat(),
        "category" => [
            vec!["Sine"; 30],
            vec!["Cosine"; 30]
        ].concat()
    ]?;

    // Create a line chart with multiple groups
    let chart = Chart::build(&df)?
        .mark_line()
        .encode((
            x("a"),
            y("b"),
            color("category"), // This creates separate lines for each category
        ))?
        .swap_axes();

    LayeredChart::new()
        .with_size(600, 400)
        .add_layer(chart)
        .save("./tests/line_3.svg")?;

    Ok(())
}

#[test]
fn test_line_4() -> Result<(), Box<dyn Error>> {
    // Create data for multiple groups (e.g., sine and cosine curves)
    let x_values: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let y_sine: Vec<f64> = x_values.iter().map(|&x| x.sin()).collect();

    // Combine data into a single DataFrame with categories
    let df = df![
        "a" => x_values,  // Repeat x values for both groups
        "b" => y_sine.clone(),
        "category" => [
            vec!["Sine"; 30],
        ].concat()
    ]?;

    // Create a line chart with multiple groups
    let chart = Chart::build(&df)?.mark_line().encode((
        x("a"),
        y("b"),
        color("category"), // This creates separate lines for each category
    ))?;

    LayeredChart::new()
        .with_size(500, 400)
        .with_legend(true)
        .add_layer(chart)
        .save("./tests/line_4.svg")?;

    Ok(())
}
