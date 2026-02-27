use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = load_dataset("iris")?;
    let df = df.select(["sepal_length", "species"])?;

    Chart::build(&df)?
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_groupby("species"),
        )?
        .mark_area()?
        .configure_area(|a| a.with_opacity(0.5))
        .encode((x("sepal_length"), y("density"), color("species")))?
        .into_layered()
        .save("./examples/density.svg")?;

    Ok(())
}
