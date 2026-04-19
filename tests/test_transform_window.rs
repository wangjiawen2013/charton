use charton::prelude::*;
use std::error::Error;

#[test]
fn tests_transform_window_1() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("iris")?;
    // Create a chart with window transform
    let chart = chart!(ds)?
        .transform_window(
            WindowTransform::new(WindowFieldDef::new(
                "sepal_length",
                WindowOnlyOp::CumeDist,
                "ecdf", // This will be the output column name
            ))
            .with_groupby("species")
            .with_normalize(false), // Normalize to [0,1] range
        )?
        .mark_line()?
        .configure_line(|l| l.with_interpolation("step")) // Add step interpolation
        .encode((
            alt::x("sepal_length"),
            alt::y("ecdf"),
            alt::color("species"),
        ))?;

    chart
        .with_size(600, 400)
        .with_title("Empirical Cumulative Distribution")
        .save("./tests/transform_window_1.svg")?;

    Ok(())
}
