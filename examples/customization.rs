use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..24).map(|index| index as f64).collect();
    let y: Vec<f64> = x.iter().map(|value| value * 1.8 + 8.0).collect();
    let group: Vec<&str> = (0..24)
        .map(|index| if index % 2 == 0 { "Even" } else { "Odd" })
        .collect();

    let chart = chart!(x, y, group)?
        .mark_point()?
        .configure_point(|point| point.with_size(5.0))
        .encode((alt::x("x"), alt::y("y"), alt::color("group")))?
        .with_size(720, 480)
        .with_margins(0.08, 0.04, 0.08, 0.04)
        .with_grid(true)
        .with_title("Scatter plot customization")
        .with_y_label("Value")
        .configure_theme(|theme| {
            theme
                .with_grid_color("#d4dddd")
                .with_grid_width(0.75)
                .with_background_color("#fbfcfc")
        });

    chart
        .clone()
        .save("docs/src/images/customization_with_legend.svg")?;

    chart
        .configure_theme(|theme| theme.with_show_legend(false).with_show_axes(false))
        .save("docs/src/images/customization_without_legend_or_axes.svg")?;

    Ok(())
}
