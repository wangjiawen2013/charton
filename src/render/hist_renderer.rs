use crate::visual::color::SingleColor;
use std::fmt::Write;

pub(crate) struct VerticalHistogramBarConfig {
    pub x_center: f64,
    pub y_zero: f64,
    pub y_value: f64,
    pub width: f64,
    pub fill_color: Option<SingleColor>,
    pub stroke_color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
}

pub(crate) struct HorizontalHistogramBarConfig {
    pub x_zero: f64,
    pub x_value: f64,
    pub y_center: f64,
    pub height: f64,
    pub fill_color: Option<SingleColor>,
    pub stroke_color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
}

/// Renders a vertical histogram bar into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `config` - Configuration parameters for the vertical histogram bar
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_vertical_histogram_bar(
    svg: &mut String,
    config: VerticalHistogramBarConfig,
) -> std::fmt::Result {
    // Calculate bar edges based on the center position
    let x_left = config.x_center - config.width / 2.0;

    // Determine the top and bottom of the bar
    let (y_top, y_bottom) = if config.y_value < config.y_zero {
        (config.y_value, config.y_zero)
    } else {
        (config.y_zero, config.y_value)
    };

    // Calculate dimensions
    let rect_width = config.width;
    let rect_height = (y_bottom - y_top).abs();

    // Determine fill color
    let fill_str = if let Some(color) = &config.fill_color {
        color.get_color()
    } else {
        "none".to_string() // Default histogram color
    };

    // Determine stroke
    let stroke_str = if let Some(color) = &config.stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Add the rectangle to the SVG
    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}" />"#,
        x_left, y_top, rect_width, rect_height, fill_str, stroke_str, config.stroke_width, config.opacity
    )
}

/// Renders a horizontal histogram bar into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `config` - Configuration parameters for the horizontal histogram bar
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_horizontal_histogram_bar(
    svg: &mut String,
    config: HorizontalHistogramBarConfig,
) -> std::fmt::Result {
    // Calculate bar edges based on the center position
    let y_top = config.y_center - config.height / 2.0;

    // Determine the left and right of the bar
    let (x_left, x_right) = if config.x_value < config.x_zero {
        (config.x_value, config.x_zero)
    } else {
        (config.x_zero, config.x_value)
    };

    // Calculate dimensions
    let rect_width = (x_right - x_left).abs();
    let rect_height = config.height;

    // Determine fill color
    let fill_str = if let Some(color) = &config.fill_color {
        color.get_color()
    } else {
        "none".to_string() // Default histogram color
    };

    // Determine stroke
    let stroke_str = if let Some(color) = &config.stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Add the rectangle to the SVG
    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}" />"#,
        x_left, y_top, rect_width, rect_height, fill_str, stroke_str, config.stroke_width, config.opacity
    )
}
