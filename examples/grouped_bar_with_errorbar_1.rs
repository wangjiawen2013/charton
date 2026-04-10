use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("penguins")?;
    println!("{:?}", ds);

    // 2. Build the Error Bar layer
    let errorbar = chart!(&ds)?
        .mark_errorbar()?
        // Mapping 'Sex' to color triggers the dodge logic
        .encode((
            alt::x("Species"),
            alt::y("Body Mass (g)"),
            alt::color("Sex"),
        ))?;

    // 3. Add a Bar layer to see the alignment
    let bar = chart!(ds)?.mark_bar()?.encode((
        alt::x("Species"),
        alt::y("Body Mass (g)").with_aggregate("mean"),
        alt::color("Sex"),
    ))?;

    // 4. Create the multiple layered Chart
    errorbar
        .and(bar)
        .with_size(600, 400)
        .with_title("Grouped Error Bars with Mean & Std Dev")
        .save("docs/src/images/grouped_bar_with_errorbar_1.svg")?;

    Ok(())
}
