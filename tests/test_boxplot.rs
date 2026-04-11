use charton::prelude::*;
use std::error::Error;

#[test]
fn test_boxplot_1() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("penguins")?;

    chart!(ds)?
        .mark_boxplot()?
        .encode((
            alt::x("Sex"),
            alt::y("Body Mass (g)"),
            alt::color("Species"),
        ))?
        .coord_flip()
        .save("./tests/boxplot_1.svg")?;

    Ok(())
}
