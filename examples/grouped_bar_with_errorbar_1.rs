use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = load_dataset("penguins")?;
    println!("{:?}", df);

    // 2. Build the Error Bar layer
    let errorbar_layer = Chart::build(&df)?
        .mark_errorbar()?
        // Mapping 'Sex' to color triggers the dodge logic
        .encode((x("Species"), y("Body Mass (g)"), color("Sex")))?;

    // 3. Add a Bar layer to see the alignment
    let bar_layer = Chart::build(&df)?.mark_bar()?.encode((
        x("Species"),
        y("Body Mass (g)").with_aggregate("mean"),
        color("Sex"),
    ))?;

    // 4. Create the Layered Chart
    LayeredChart::new()
        .with_size(600, 400)
        .with_title("Grouped Error Bars with Mean & Std Dev")
        .add_layer(errorbar_layer) // Error bars on bottom
        .add_layer(bar_layer) // Layer bars
        .save("docs/src/images/grouped_bar_with_errorbar_1.svg")?;

    Ok(())
}
