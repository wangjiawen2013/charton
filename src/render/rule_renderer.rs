use crate::visual::color::SingleColor;
use std::fmt::Write;

/// Renders a vertical rule (line) into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `x` - X-coordinate of the rule
/// * `y1` - Starting Y-coordinate of the rule
/// * `y2` - Ending Y-coordinate of the rule
/// * `stroke_color` - Stroke color for the rule
/// * `stroke_width` - Width of the stroke
/// * `opacity` - Opacity level (0.0 to 1.0)
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_vertical_rule(
    svg: &mut String,
    x: f64,
    y1: f64,
    y2: f64,
    stroke_color: &Option<SingleColor>,
    stroke_width: f64,
    opacity: f64,
) -> std::fmt::Result {
    // Convert ColorScheme to string values for SVG
    let stroke_str = if let Some(color) = stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
        x, y1, x, y2, stroke_str, stroke_width, opacity
    )
}

/// Renders a horizontal rule (line) into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `x1` - Starting X-coordinate of the rule
/// * `x2` - Ending X-coordinate of the rule
/// * `y` - Y-coordinate of the rule
/// * `stroke_color` - Stroke color for the rule
/// * `stroke_width` - Width of the stroke
/// * `opacity` - Opacity level (0.0 to 1.0)
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_horizontal_rule(
    svg: &mut String,
    x1: f64,
    x2: f64,
    y: f64,
    stroke_color: &Option<SingleColor>,
    stroke_width: f64,
    opacity: f64,
) -> std::fmt::Result {
    // Convert ColorScheme to string values for SVG
    let stroke_str = if let Some(color) = stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
        x1, y, x2, y, stroke_str, stroke_width, opacity
    )
}
