#[cfg(feature = "bridge")]
use charton::prelude::*;
#[cfg(feature = "bridge")]
use polars::prelude::*;
#[cfg(feature = "bridge")]
use std::error::Error;

#[cfg(feature = "bridge")]
fn main() -> Result<(), Box<dyn Error>> {
    // Set the path to your Python executable on windows/linux/macOS
    let exe_path = r"where-is-my/python";
    let df1 = df![
        "Model" => ["S1", "M1", "R2", "P8", "M4", "T5", "V1"],
        "Price" => [2430, 3550, 5700, 8750, 2315, 3560, 980],
        "Discount" => [Some(0.65), Some(0.73), Some(0.82), None, Some(0.51), None, Some(0.26)],
    ]?;

    // Code for plotting with Altair
    let raw_plotting_code = r#"
import altair as alt

chart = alt.Chart(df1).mark_point().encode(
    x='Price',
    y='Discount',
    color='Model',
).properties(width=200, height=200)
"#;
    Plot::<Altair>::build(data!(&df1)?)?
        .with_exe_path(exe_path)?
        .with_plotting_code(raw_plotting_code)
        .save("scatter.svg")?;

    Ok(())
}

#[cfg(not(feature = "bridge"))]
fn main() {
    println!("This example requires --features \"polars altair\" to run.");
}
