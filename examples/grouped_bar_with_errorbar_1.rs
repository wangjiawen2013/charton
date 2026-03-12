use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = load_dataset("penguins")?;
    println!("{:?}", df);

    // 2. Build the Error Bar layer
    let errorbar = Chart::build(&df)?
        .mark_errorbar()?
        // Mapping 'Sex' to color triggers the dodge logic
        .encode((x("Species"), y("Body Mass (g)"), color("Sex")))?;

    // 3. Add a Bar layer to see the alignment
    let bar = Chart::build(&df)?.mark_bar()?.encode((
        x("Species"),
        y("Body Mass (g)").with_aggregate("mean"),
        color("Sex"),
    ))?;

    // 4. Create the multiple layered Chart
    errorbar
        .and(bar)
        .with_size(600, 400)
        .with_title("Grouped Error Bars with Mean & Std Dev")
        .save("docs/src/images/grouped_bar_with_errorbar_1.svg")?;

    Ok(())
}
