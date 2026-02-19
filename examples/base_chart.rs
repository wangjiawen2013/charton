use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Prepare Source Data
    let df = df![
        "length" => [4.4, 4.6, 4.7, 4.9, 5.0, 5.1, 5.4],
        "width" => [2.9, 3.1, 3.2, 3.0, 3.6, 3.5, 3.9]
    ]?;

    // 2. Define the Base Specification
    // We create a Chart<NoMark> that holds the shared data and encoding logic.
    // Validation is deferred here because no specific mark is assigned yet.
    let base = Chart::build(&df)?
        .encode((
            x("length"),
            y("width"),
        ))?;

    // 3. Derive the Line Layer from Base
    // We clone the base and 'specialize' it into a Line chart.
    // The .mark_line() call triggers validation of the existing encodings.
    let line_layer = base.clone()
        .mark_line()?;

    // 4. Derive the Scatter Layer from Base
    // Again, we specialize the base, but this time into a Point chart.
    // This demonstrates the "one-to-many" capability of the Base Pattern.
    let scatter_layer = base
        .mark_point()?
        .configure_point(|p| p.with_color("red").with_size(6.0));

    // 5. Assemble into a Layered Composition
    // The LayeredChart acts as a container for these specialized specs.
    let chart = LayeredChart::new()
        .add_layer(line_layer)
        .add_layer(scatter_layer);

    // 6. Export the final visualization
    chart.save("./base_pattern_example.svg")?;

    Ok(())
}