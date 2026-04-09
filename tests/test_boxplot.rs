use charton::prelude::*;
use std::error::Error;

#[test]
fn test_boxplot_1() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("penguins")?;

    chart!(ds)?
        .mark_boxplot()?
        .encode((
            alt::x("island"),
            alt::y("body_mass_g"),
            alt::color("species"),
        ))?
        .coord_flip()
        .save("./tests/boxplot_1.svg")?;

    Ok(())
}
