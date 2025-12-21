use crate::visual::color::SingleColor;
use std::fmt::Write;

/// Renders a vertical bar into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `x_center` - X-coordinate of the bar center
/// * `y_zero` - Y-coordinate of the zero line
/// * `y_value` - Y-coordinate of the data value
/// * `width` - Width of the bar
/// * `fill_color` - Fill color for the bar
/// * `stroke_color` - Stroke color for the bar
/// * `stroke_width` - Width of the stroke
/// * `opacity` - Opacity level (0.0 to 1.0)
///
/// # Returns
/// Result indicating success or failure of the operation
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_vertical_bar(
    svg: &mut String,
    x_center: f64,
    y_zero: f64,
    y_value: f64,
    width: f64,
    fill_color: &Option<SingleColor>,
    stroke_color: &Option<SingleColor>,
    stroke_width: f64,
    opacity: f64,
) -> std::fmt::Result {
    // Calculate bar edges based on the center position
    let x_left = x_center - width / 2.0;

    // Determine the top and bottom of the bar
    let (y_top, y_bottom) = if y_value < y_zero {
        (y_value, y_zero)
    } else {
        (y_zero, y_value)
    };

    // Calculate dimensions
    let rect_width = width;
    let rect_height = (y_bottom - y_top).abs();

    // Determine fill color
    let fill_str = if let Some(color) = fill_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Determine stroke
    let stroke_str = if let Some(color) = stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Add the rectangle to the SVG
    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}" />"#,
        x_left, y_top, rect_width, rect_height, fill_str, stroke_str, stroke_width, opacity
    )
}

/// Renders a horizontal bar into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `x_zero` - X-coordinate of the zero line
/// * `x_value` - X-coordinate of the data value
/// * `y_center` - Y-coordinate of the bar center
/// * `height` - Height of the bar
/// * `fill_color` - Fill color for the bar
/// * `stroke_color` - Stroke color for the bar
/// * `stroke_width` - Width of the stroke
/// * `opacity` - Opacity level (0.0 to 1.0)
///
/// # Returns
/// Result indicating success or failure of the operation
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_horizontal_bar(
    svg: &mut String,
    x_zero: f64,
    x_value: f64,
    y_center: f64,
    height: f64,
    fill_color: &Option<SingleColor>,
    stroke_color: &Option<SingleColor>,
    stroke_width: f64,
    opacity: f64,
) -> std::fmt::Result {
    // Calculate bar edges based on the center position
    let y_top = y_center - height / 2.0;

    // Determine the left and right of the bar
    let (x_left, x_right) = if x_value < x_zero {
        (x_value, x_zero)
    } else {
        (x_zero, x_value)
    };

    // Calculate dimensions
    let rect_width = (x_right - x_left).abs();
    let rect_height = height;

    // Determine fill color
    let fill_str = if let Some(color) = fill_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Determine stroke
    let stroke_str = if let Some(color) = stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Add the rectangle to the SVG
    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}" />"#,
        x_left, y_top, rect_width, rect_height, fill_str, stroke_str, stroke_width, opacity
    )
}
