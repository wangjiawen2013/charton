use crate::visual::color::SingleColor;
use std::fmt::Write;

/// Interpolation methods for line paths
#[derive(Debug, Clone, Default)]
pub enum PathInterpolation {
    /// Straight line segments between points (default)
    #[default]
    Linear,
    /// Step function that holds value until next point (appropriate for ECDF)
    StepAfter,
    /// Step function that jumps to next value immediately
    StepBefore,
}

/// Renders a line as an SVG path element and appends it to the provided SVG string.
///
/// This function takes a series of points and renders them as a line using the specified
/// interpolation method. The line is styled with the given color, stroke width, and opacity.
///
/// # Arguments
///
/// * `svg` - A mutable reference to a String where the generated SVG path element will be appended.
/// * `points` - A slice of (x, y) coordinate tuples representing the points of the line.
/// * `color` - An optional SingleColor specifying the stroke color. If None, defaults to black.
/// * `stroke_width` - The width of the line stroke as a floating point number.
/// * `opacity` - The opacity of the line as a floating point number between 0.0 and 1.0.
/// * `interpolation` - A PathInterpolation enum specifying how to connect the points.
///
/// # Returns
///
/// Returns a `std::fmt::Result` indicating success or failure of the write operation.
pub(crate) fn render_line(
    svg: &mut String,
    points: &[(f64, f64)],
    color: &Option<SingleColor>,
    stroke_width: f64,
    opacity: f64,
    interpolation: &PathInterpolation,
) -> std::fmt::Result {
    if points.is_empty() {
        return Ok(());
    }

    let color_str = if let Some(color) = color {
        color.get_color()
    } else {
        "black".to_string()
    };

    let path_data = match interpolation {
        PathInterpolation::Linear => generate_linear_path(points),
        PathInterpolation::StepAfter => generate_step_after_path(points),
        PathInterpolation::StepBefore => generate_step_before_path(points),
    };

    writeln!(
        svg,
        r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" opacity="{}" stroke-linejoin="round" stroke-linecap="round"/>"#,
        path_data, color_str, stroke_width, opacity
    )
}

// Generate SVG path data for linear interpolation
fn generate_linear_path(points: &[(f64, f64)]) -> String {
    let mut path = format!(
        "M {} {}",
        format_coordinate(points[0].0),
        format_coordinate(points[0].1)
    );
    let mut prev_point = points[0];

    for &point in &points[1..] {
        // Skip duplicate consecutive points in case of duplicated start or end points
        if point != prev_point {
            path.push_str(&format!(
                " L {} {}",
                format_coordinate(point.0),
                format_coordinate(point.1)
            ));
            prev_point = point;
        }
    }
    path
}

// Generate SVG path data for step-after interpolation (appropriate for ECDF)
fn generate_step_after_path(points: &[(f64, f64)]) -> String {
    let mut path = format!(
        "M {} {}",
        format_coordinate(points[0].0),
        format_coordinate(points[0].1)
    );
    let mut prev_point = points[0];

    for &point in &points[1..] {
        // Skip duplicate consecutive points in case of duplicated start or end points
        if point != prev_point {
            path.push_str(&format!(
                " H {} V {}",
                format_coordinate(point.0),
                format_coordinate(point.1)
            ));
            prev_point = point;
        }
    }
    path
}

// Generate SVG path data for step-before interpolation
fn generate_step_before_path(points: &[(f64, f64)]) -> String {
    let mut path = format!(
        "M {} {}",
        format_coordinate(points[0].0),
        format_coordinate(points[0].1)
    );
    let mut prev_point = points[0];

    for &point in &points[1..] {
        // Skip duplicate consecutive points in case of duplicated start or end points
        if point != prev_point {
            path.push_str(&format!(
                " V {} H {}",
                format_coordinate(point.1),
                format_coordinate(point.0)
            ));
            prev_point = point;
        }
    }
    path
}

// Format coordinate with adaptive precision
fn format_coordinate(value: f64) -> String {
    // For very large or very small numbers, use scientific notation
    if value.abs() >= 1e10 || (value.abs() < 1e-4 && value != 0.0) {
        format!("{:.6e}", value)
    } else {
        // For normal ranges, use regular formatting with trimming
        let s = format!("{:.10}", value);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
