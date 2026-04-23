#[cfg(feature = "bridge")]
use charton::prelude::*;
#[cfg(feature = "bridge")]
use polars::prelude::*;
#[cfg(feature = "bridge")]
use std::error::Error;

#[cfg(feature = "bridge")]
fn main() -> Result<(), Box<dyn Error>> {
    let exe_path = r"F:\Programs\miniconda3\envs\cellpy\python.exe";
    let iris = load_dataset("iris")?;

    let raw_plotting_code = r#"
import altair as alt

features = [
    'sepal_length',
    'sepal_width',
]

chart = alt.Chart(iris).mark_circle().encode(
    alt.X(alt.repeat("column"), type="quantitative"),
    alt.Y(alt.repeat("row"), type="quantitative"),
    color='species'
).properties(
    width=130,
    height=105
).repeat(
    row=features,
    column=features
)
"#;
    Plot::<Altair>::build(data!(&iris)?)?
        .with_exe_path(exe_path)?
        .with_plotting_code(raw_plotting_code)
        .save("docs/src/images/altair.svg")?;

    Ok(())
}

#[cfg(not(feature = "bridge"))]
fn main() {
    println!("This example requires --features \"polars altair\" to run.");
}
