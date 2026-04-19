use charton::prelude::*;
use std::error::Error;

#[test]
fn test_line_1() -> Result<(), Box<dyn Error>> {
    // Create sample data frame with a, b, and category columns
    let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let b = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let category = vec!["A", "B", "A", "B", "C"];

    // Create a point chart with only a, b, and color encodings
    chart!(a, b, category)?
        .mark_line()?
        .encode((
            alt::x("a"),
            alt::y("b"),
            //alt::color("category"),
        ))?
        .with_size(500, 300)
        .save("./tests/line_1.svg")?;

    Ok(())
}

#[test]
fn test_line_2() -> Result<(), Box<dyn Error>> {
    // Create 30 points along a sine curve
    let a: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let b: Vec<f64> = a.iter().map(|&x| x.sin()).collect();
    let category = ["Sine"; 30];

    // Create a line chart with LOESS smoothing
    chart!(a, b, category)?
        .mark_line()?
        // Apply LOESS smoothing with bandwidth 0.3
        .configure_line(|l| l.with_loess(true).with_loess_bandwidth(0.3))
        .encode((alt::x("a"), alt::y("b"), alt::color("category")))?
        .with_size(600, 400)
        .save("./tests/line_2.svg")?;

    Ok(())
}

#[test]
fn test_line_3() -> Result<(), Box<dyn Error>> {
    // Create data for multiple groups (e.g., sine and cosine curves)
    let x_values: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let y_sine: Vec<f64> = x_values.iter().map(|&x| x.sin()).collect();
    let y_cosine: Vec<f64> = x_values.iter().map(|&x| x.cos()).collect();

    let a = x_values.repeat(2);
    let b = [y_sine, y_cosine].concat();
    let category = [vec!["Sine"; 30], vec!["Cosine"; 30]].concat();

    // Create a line chart with multiple groups
    chart!(a, b, category)?
        .mark_line()?
        .encode((
            alt::x("a"),
            alt::y("b"),
            alt::color("category"), // This creates separate lines for each category
        ))?
        .with_size(600, 400)
        .coord_flip()
        .save("./tests/line_3.svg")?;

    Ok(())
}

#[test]
fn test_line_4() -> Result<(), Box<dyn Error>> {
    // Create data for multiple groups (e.g., sine and cosine curves)
    let a: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let b: Vec<f64> = a.iter().map(|&x| x.sin()).collect();
    let category = ["Sine"; 30];

    // Create a line chart with multiple groups
    chart!(a, b, category)?
        .mark_line()?
        .encode((
            alt::x("a"),
            alt::y("b"),
            alt::color("category"), // This creates separate lines for each category
        ))?
        .with_size(500, 400)
        .save("./tests/line_4.svg")?;

    Ok(())
}
