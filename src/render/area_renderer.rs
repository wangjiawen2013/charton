use crate::visual::color::SingleColor;
use std::fmt::Write;

pub(crate) struct AreaConfig {
    pub points: Vec<(f64, f64)>,
    pub fill_color: Option<SingleColor>,
    pub stroke_color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
    pub closed: bool,
}

/// Renders an area into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `config` - Configuration parameters for the area
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_area(svg: &mut String, config: AreaConfig) -> std::fmt::Result {
    // Build path data for the area
    let mut path_data = String::new();

    // Move to the first point
    let first_point = &config.points[0];
    path_data.push_str(&format!("M {} {}", first_point.0, first_point.1));

    // Draw line to each subsequent point
    for p in config.points.iter().skip(1) {
        path_data.push_str(&format!(" L {} {}", p.0, p.1));
    }

    // If closed, close the path
    if config.closed {
        path_data.push_str(" Z");
    }

    // Determine fill color
    let fill_str = if let Some(color) = &config.fill_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Determine stroke
    let stroke_str = if let Some(color) = &config.stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Add the path to the SVG
    writeln!(
        svg,
        r#"<path d="{}" fill="{}" stroke="{}" stroke-width="{}" fill-opacity="{}" />"#,
        path_data, fill_str, stroke_str, config.stroke_width, config.opacity
    )
}
