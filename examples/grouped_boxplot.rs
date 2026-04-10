use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("penguins")?;
    println!("{}", &ds);

    chart!(ds)?
        .mark_boxplot()?
        .encode((
            alt::x("island"),
            alt::y("body_mass_g"),
            alt::color("species"),
        ))?
        .save("docs/src/images/grouped_boxplot.svg")?;

    Ok(())
}
