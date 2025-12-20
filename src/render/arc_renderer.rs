use crate::visual::color::SingleColor;
use std::fmt::Write;

/// Renders an arc slice (a pie/wedge shape) into the SVG string
///
/// This function calculates the path for an arc slice based on center coordinates,
/// radius, and start/end angles, then appends the corresponding SVG path element
/// to the provided string.
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `center_x` - X-coordinate of the arc's center
/// * `center_y` - Y-coordinate of the arc's center
/// * `radius` - Radius of the arc
/// * `inner_radius_ratio` - Ratio of inner radius to outer radius (0.0 to 1.0)
/// * `start_angle` - Starting angle in radians
/// * `end_angle` - Ending angle in radians
/// * `fill_color` - Reference to the fill color
/// * `stroke_color` - Reference to the stroke color
/// * `stroke_width` - Width of the stroke
/// * `opacity` - Opacity of the arc
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_arc_slice(
    svg: &mut String,
    center_x: f64,
    center_y: f64,
    radius: f64,
    inner_radius_ratio: f64,
    start_angle: f64,
    end_angle: f64,
    fill_color: &Option<SingleColor>,
    stroke_color: &Option<SingleColor>,
    stroke_width: f64,
    opacity: f64,
) -> std::fmt::Result {
    // Determine inner radius based on mark settings
    let inner_radius = radius * inner_radius_ratio;

    // Calculate points
    let cos_start = start_angle.cos();
    let sin_start = start_angle.sin();
    let cos_end = end_angle.cos();
    let sin_end = end_angle.sin();

    // Outer arc points
    let outer_start_x = center_x + radius * cos_start;
    let outer_start_y = center_y + radius * sin_start;
    let outer_end_x = center_x + radius * cos_end;
    let outer_end_y = center_y + radius * sin_end;

    // Determine if this is a large arc (greater than 180 degrees)
    let large_arc_flag = if end_angle - start_angle > std::f64::consts::PI {
        1
    } else {
        0
    };

    let fill_color_str = fill_color
        .as_ref()
        .map(|c| c.get_color())
        .unwrap_or_else(|| "none".to_string());

    let stroke_str = if let Some(stroke) = stroke_color {
        stroke.get_color()
    } else {
        "none".to_string()
    };

    if inner_radius > 0.0 {
        // Donut slice - create a path with both outer and inner arcs
        let inner_start_x = center_x + inner_radius * cos_start;
        let inner_start_y = center_y + inner_radius * sin_start;
        let inner_end_x = center_x + inner_radius * cos_end;
        let inner_end_y = center_y + inner_radius * sin_end;

        // Create the path for the donut slice
        writeln!(
            svg,
            r#"<path d="M {} {} A {} {} 0 {} 1 {} {} L {} {} A {} {} 0 {} 0 {} {} L {} {} Z" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
            outer_start_x,
            outer_start_y, // Move to outer start
            radius,
            radius,         // Outer arc radii
            large_arc_flag, // Large arc flag
            outer_end_x,
            outer_end_y, // Outer arc end point
            inner_end_x,
            inner_end_y, // Line to inner end
            inner_radius,
            inner_radius,   // Inner arc radii
            large_arc_flag, // Large arc flag (same as outer)
            inner_start_x,
            inner_start_y, // Inner arc end point (which is start of outer)
            outer_start_x,
            outer_start_y, // Line back to outer start
            fill_color_str,
            stroke_str,
            stroke_width,
            opacity,
        )
    } else {
        // Regular pie slice
        writeln!(
            svg,
            r#"<path d="M {} {} L {} {} A {} {} 0 {} 1 {} {} L {} {} Z" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
            center_x,
            center_y, // Move to center
            outer_start_x,
            outer_start_y, // Line to outer start
            radius,
            radius,         // Arc radii
            large_arc_flag, // Large arc flag
            outer_end_x,
            outer_end_y, // Arc end point
            center_x,
            center_y, // Line back to center
            fill_color_str,
            stroke_str,
            stroke_width,
            opacity,
        )
    }
}
