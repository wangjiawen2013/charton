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

pub(crate) struct LineConfig {
    pub points: Vec<(f64, f64)>,
    pub color: SingleColor,
    pub stroke_width: f64,
    pub opacity: f64,
    pub interpolation: PathInterpolation,
}

/// Renders a line as an SVG path element and appends it to the provided SVG string.
///
/// This function takes a series of points and renders them as a line using the specified
/// interpolation method. The line is styled with the given color, stroke width, and opacity.
///
/// # Arguments
///
/// * `svg` - A mutable reference to a String where the generated SVG path element will be appended.
/// * `config` - Configuration parameters for the line
///
/// # Returns
///
/// Returns a `std::fmt::Result` indicating success or failure of the write operation.
pub(crate) fn render_line(svg: &mut String, config: LineConfig) -> std::fmt::Result {
    if config.points.is_empty() {
        return Ok(());
    }

    let path_data = match config.interpolation {
        PathInterpolation::Linear => generate_linear_path(&config.points),
        PathInterpolation::StepAfter => generate_step_after_path(&config.points),
        PathInterpolation::StepBefore => generate_step_before_path(&config.points),
    };

    writeln!(
        svg,
        r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" opacity="{}" stroke-linejoin="round" stroke-linecap="round"/>"#,
        path_data, config.color.as_str(), config.stroke_width, config.opacity
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
