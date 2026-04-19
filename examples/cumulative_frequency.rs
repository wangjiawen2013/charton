use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("iris")?;
    // Create a chart with window transform
    chart!(ds)?
        .transform_window(
            WindowTransform::new(WindowFieldDef::new(
                "sepal_length",
                WindowOnlyOp::CumeDist,
                "ecdf", // This will be the output column name
            ))
            .with_groupby("species")
            .with_normalize(false),
        )?
        .mark_line()?
        .configure_line(|l| l.with_interpolation("step")) // Add step interpolation
        .encode((
            alt::x("sepal_length"),
            alt::y("ecdf"),
            alt::color("species"),
        ))?
        .with_title("Empirical Cumulative Distribution")
        .save("docs/src/images/cumulative_frequency.svg")?;

    Ok(())
}
