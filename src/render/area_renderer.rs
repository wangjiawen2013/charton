use crate::visual::color::SingleColor;
use std::fmt::Write;

/// Renders an area into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `points` - Vector of (x, y) coordinate pairs defining the area
/// * `fill_color` - Fill color for the area
/// * `stroke_color` - Stroke color for the area
/// * `stroke_width` - Width of the stroke
/// * `opacity` - Opacity level (0.0 to 1.0)
/// * `closed` - Whether to close the path (for area charts)
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_area(
    svg: &mut String,
    points: &[(f64, f64)],
    fill_color: &Option<SingleColor>,
    stroke_color: &Option<SingleColor>,
    stroke_width: f64,
    opacity: f64,
    closed: bool,
) -> std::fmt::Result {
    // Build path data for the area
    let mut path_data = String::new();

    // Move to the first point
    path_data.push_str(&format!("M {} {}", points[0].0, points[0].1));

    // Draw line to each subsequent point
    for p in points.iter().skip(1) {
        path_data.push_str(&format!(" L {} {}", p.0, p.1));
    }

    // If closed, close the path
    if closed {
        path_data.push_str(" Z");
    }

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

    // Add the path to the SVG
    writeln!(
        svg,
        r#"<path d="{}" fill="{}" stroke="{}" stroke-width="{}" fill-opacity="{}" />"#,
        path_data, fill_str, stroke_str, stroke_width, opacity
    )
}
