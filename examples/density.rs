use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ds = load_dataset("iris")?;

    chart!(ds)?
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_groupby("species"),
        )?
        .mark_area()?
        .configure_area(|a| a.with_opacity(0.5))
        .encode((
            alt::x("sepal_length"),
            alt::y("density"),
            alt::color("species"),
        ))?
        .save("docs/src/images/density.svg")?;

    Ok(())
}
