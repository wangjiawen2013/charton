use crate::visual::color::SingleColor;
use std::fmt::Write;

pub(crate) struct ArcSliceConfig {
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
    pub inner_radius_ratio: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub fill_color: Option<SingleColor>,
    pub stroke_color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
}

/// Renders an arc slice (a pie/wedge shape) into the SVG string
///
/// This function calculates the path for an arc slice based on center coordinates,
/// radius, and start/end angles, then appends the corresponding SVG path element
/// to the provided string.
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `config` - Configuration parameters for the arc slice
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_arc_slice(svg: &mut String, config: ArcSliceConfig) -> std::fmt::Result {
    // Determine inner radius based on mark settings
    let inner_radius = config.radius * config.inner_radius_ratio;

    // Calculate points
    let cos_start = config.start_angle.cos();
    let sin_start = config.start_angle.sin();
    let cos_end = config.end_angle.cos();
    let sin_end = config.end_angle.sin();

    // Outer arc points
    let outer_start_x = config.center_x + config.radius * cos_start;
    let outer_start_y = config.center_y + config.radius * sin_start;
    let outer_end_x = config.center_x + config.radius * cos_end;
    let outer_end_y = config.center_y + config.radius * sin_end;

    // Determine if this is a large arc (greater than 180 degrees)
    let large_arc_flag = if config.end_angle - config.start_angle > std::f64::consts::PI {
        1
    } else {
        0
    };

    let fill_color_str = config
        .fill_color
        .as_ref()
        .map(|c| c.get_color())
        .unwrap_or_else(|| "none".to_string());

    let stroke_str = if let Some(stroke) = &config.stroke_color {
        stroke.get_color()
    } else {
        "none".to_string()
    };

    if inner_radius > 0.0 {
        // Donut slice - create a path with both outer and inner arcs
        let inner_start_x = config.center_x + inner_radius * cos_start;
        let inner_start_y = config.center_y + inner_radius * sin_start;
        let inner_end_x = config.center_x + inner_radius * cos_end;
        let inner_end_y = config.center_y + inner_radius * sin_end;

        // Create the path for the donut slice
        writeln!(
            svg,
            r#"<path d="M {} {} A {} {} 0 {} 1 {} {} L {} {} A {} {} 0 {} 0 {} {} L {} {} Z" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
            outer_start_x,
            outer_start_y, // Move to outer start
            config.radius,
            config.radius,  // Outer arc radii
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
            config.stroke_width,
            config.opacity,
        )
    } else {
        // Regular pie slice
        writeln!(
            svg,
            r#"<path d="M {} {} L {} {} A {} {} 0 {} 1 {} {} L {} {} Z" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
            config.center_x,
            config.center_y, // Move to center
            outer_start_x,
            outer_start_y, // Line to outer start
            config.radius,
            config.radius,  // Arc radii
            large_arc_flag, // Large arc flag
            outer_end_x,
            outer_end_y, // Arc end point
            config.center_x,
            config.center_y, // Line back to center
            fill_color_str,
            stroke_str,
            config.stroke_width,
            config.opacity,
        )
    }
}
