use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let month = vec![
        "Jan", "Jan", "Jan", "Jan", "Feb", "Feb", "Feb", "Feb", "Mar", "Mar", "Mar", "Mar",
    ];
    let revenue = vec![
        100.0, -120.1, 90.0, -140.0, 110.0, 130.0, -100.0, 120.0, 90.0, 140.0, -110.0, -130.0,
    ];
    let region = vec![
        "North", "South", "East", "West", "North", "South", "East", "West", "North", "South",
        "East", "West",
    ];

    // Create a bar chart with color encoding
    chart!(&month, &revenue, &region)?
        .mark_bar()?
        .configure_bar(|b| {
            b.with_stroke("black")
                .with_stroke_width(1.0)
                .with_width(0.5)
        })
        .encode((
            alt::x("month"),
            alt::y("revenue").with_stack("none"),
            alt::color("region"),
        ))?
        .coord_flip()
        .save("docs/src/images/swapped_axes.svg")?;

    Ok(())
}
