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
            .with_groupbys(["species"])
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
        .save("./target/test-output/transform_window_1.svg")?;

    Ok(())
}

/// Cumulative sum accumulates in row order without sorting. It is the statistic
/// behind a waterfall's running total.
#[test]
fn tests_transform_window_cumulative_sum() -> Result<(), Box<dyn Error>> {
    let step = ["Start", "Q1", "Q2", "Q3"];
    let delta = [100.0, 40.0, -25.0, 15.0];

    chart!(step, delta)?
        .transform_window(WindowTransform::new(WindowFieldDef::new(
            "delta",
            WindowOnlyOp::CumulativeSum,
            "total",
        )))?
        .mark_line()?
        .encode((alt::x("step"), alt::y("total")))?
        .save("./target/test-output/transform_window_cumsum.svg")?;

    Ok(())
}
