use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("penguins")?;
    println!("{:?}", &ds);

    chart!(ds)?
        .mark_boxplot()?
        .encode((
            alt::x("Sex"),
            alt::y("Body Mass (g)"),
            alt::color("Species"),
        ))?
        .save("docs/src/images/grouped_boxplot.svg")?;

    Ok(())
}
