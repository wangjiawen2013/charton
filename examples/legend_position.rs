use charton::core::guide::LegendPosition;
use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..18).map(|index| index as f64).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|value| 20.0 + value * 1.5 + (value * 0.8).sin() * 4.0)
        .collect();
    let group: Vec<&str> = (0..18)
        .map(|index| match index % 3 {
            0 => "North",
            1 => "Central",
            _ => "South",
        })
        .collect();

    let categorical_chart = chart!(x, y, group)?
        .mark_point()?
        .configure_point(|point| point.with_size(7.0))
        .encode((alt::x("x"), alt::y("y"), alt::color("group")))?
        .with_size(720, 480)
        .with_x_label("Observation")
        .with_y_label("Value")
        .configure_theme(|theme| {
            theme
                .with_palette(["#17648d", "#d06b38", "#4c956c"])
                .with_legend_label_color("#30434f")
                .with_legend_title_color("#1b252c")
                .with_legend_block_gap(18.0)
                .with_legend_marker_text_gap(8.0)
        });

    categorical_chart
        .clone()
        .configure_theme(|theme| theme.with_legend_position(LegendPosition::Right))
        .save("docs/src/images/legend_position_right.svg")?;

    categorical_chart
        .clone()
        .configure_theme(|theme| theme.with_legend_position(LegendPosition::Left))
        .save("docs/src/images/legend_position_left.svg")?;

    categorical_chart
        .clone()
        .configure_theme(|theme| theme.with_legend_position(LegendPosition::Top))
        .save("docs/src/images/legend_position_top.svg")?;

    categorical_chart
        .clone()
        .configure_theme(|theme| theme.with_legend_position(LegendPosition::Bottom))
        .save("docs/src/images/legend_position_bottom.svg")?;

    categorical_chart
        .configure_theme(|theme| theme.with_show_legend(false).with_show_axes(false))
        .save("docs/src/images/legend_position_hidden.svg")?;

    let color_value: Vec<f64> = (0..18).map(|index| index as f64).collect();
    let colorbar_chart = chart!(color_value, y)?
        .mark_point()?
        .configure_point(|point| point.with_size(7.0))
        .encode((
            alt::x("color_value"),
            alt::y("y"),
            alt::color("color_value"),
        ))?
        .with_size(720, 480)
        .with_x_label("Observation")
        .with_y_label("Value")
        .configure_theme(|theme| {
            theme
                .with_color_map(ColorMap::Viridis)
                .with_legend_position(LegendPosition::Top)
                .with_legend_label_size(11.0)
                .with_legend_margin(12.0)
        });

    colorbar_chart.save("docs/src/images/legend_position_colorbar_top.svg")?;

    Ok(())
}
