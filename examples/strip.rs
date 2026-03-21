use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = load_dataset("iris")?;

    let chart = Chart::build(&df)?
        .mark_tick()?
        .encode((x("sepal_width"), y("species"), color("species")))?
        .configure_tick(|m| {
            m.with_thickness(2.0)
                .with_band_size(10.0)
                .with_color("blue")
        });
    chart.save("docs/src/images/strip.svg")?;

    Ok(())
}
