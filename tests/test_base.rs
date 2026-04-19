use charton::prelude::*;
use std::error::Error;

#[test]
fn test_base() -> Result<(), Box<dyn Error>> {
    // 1. Prepare Source Data
    let length = [4.4, 4.6, 4.7, 4.9, 5.0, 5.1, 5.4];
    let width = [2.9, 3.1, 3.2, 3.0, 3.6, 3.5, 3.9];

    // 2. Define the Base Specification
    // We create a Chart<NoMark> that holds the shared data and encoding logic.
    // Validation is deferred here because no specific mark is assigned yet.
    let base = chart!(length, width)?.encode((alt::x("length"), alt::y("width")))?;

    // 3. Derive the Line Layer from Base
    // We clone the base and 'specialize' it into a Line chart.
    // The .mark_line() call triggers validation of the existing encodings.
    let line = base.clone().mark_line()?;

    // 4. Derive the Scatter Layer from Base
    // Again, we specialize the base, but this time into a Point chart.
    // This demonstrates the "one-to-many" capability of the Base Pattern.
    let scatter = base.mark_point()?;

    // 5. Assemble into a Layered Composition
    // The LayeredChart acts as a container for these specialized specs.
    let chart = line.and(scatter);

    // 6. Export the final visualization
    chart.save("./tests/base.svg")?;

    Ok(())
}
