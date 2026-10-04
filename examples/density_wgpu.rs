use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ds = load_dataset("iris")?;

    chart!(ds)?
        .mark_density()?
        .configure_density(|density| density.with_opacity(0.5))
        .encode((alt::x("sepal_length"), alt::color("species")))?
        .save("target/example-output/density_gpu.png")?;

    Ok(())
}
