use crate::visual::{color::SingleColor, shape::PointShape};
use std::fmt::Write;

pub(crate) struct PointConfig {
    pub cx: f64,
    pub cy: f64,
    pub fill_color: Option<SingleColor>,
    pub shape: PointShape,
    pub size: f64,
    pub opacity: f64,
    pub stroke_color: Option<SingleColor>,
    pub stroke_width: f64,
}

/// Renders a point with all its visual properties into the SVG string.
/// This is a simplified version for legend rendering.
///
/// # Parameters
///
/// - `svg`: A mutable reference to a string used to accumulate the generated SVG content.
/// - `config`: Configuration parameters for the point
///
/// # Returns
///
/// Returns a `std::fmt::Result` indicating success or failure of the write operation.
pub(crate) fn render_point(svg: &mut String, config: PointConfig) -> std::fmt::Result {
    // Convert ColorScheme to string values for SVG
    let fill_str = if let Some(color) = &config.fill_color {
        color.get_color()
    } else {
        "none".to_string()
    };
    let stroke_str = if let Some(color) = &config.stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    match config.shape {
        // Basic shapes implementation
        PointShape::Circle => {
            writeln!(
                svg,
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                config.cx,
                config.cy,
                config.size,
                fill_str,
                stroke_str,
                config.stroke_width,
                config.opacity
            )
        }
        PointShape::Square => {
            writeln!(
                svg,
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                config.cx - config.size,
                config.cy - config.size,
                config.size * 2.0,
                config.size * 2.0,
                fill_str,
                stroke_str,
                config.stroke_width,
                config.opacity
            )
        }
        PointShape::Diamond => {
            writeln!(
                svg,
                r#"<polygon points="{} {} {} {} {} {} {} {}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                config.cx,
                config.cy - config.size, // Top
                config.cx + config.size,
                config.cy, // Right
                config.cx,
                config.cy + config.size, // Bottom
                config.cx - config.size,
                config.cy, // Left
                fill_str,
                stroke_str,
                config.stroke_width,
                config.opacity
            )
        }
        PointShape::Triangle => {
            let height = config.size * 1.732; // Height of equilateral triangle
            writeln!(
                svg,
                r#"<polygon points="{} {} {} {} {} {}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                config.cx,
                config.cy - height * 2.0 / 3.0, // Top
                config.cx - config.size,
                config.cy + height / 3.0, // Bottom left
                config.cx + config.size,
                config.cy + height / 3.0, // Bottom right
                fill_str,
                stroke_str,
                config.stroke_width,
                config.opacity
            )
        }
        PointShape::Pentagon => {
            let points = (0..5)
                .map(|i| {
                    let angle =
                        std::f64::consts::PI / 2.0 + i as f64 * 2.0 * std::f64::consts::PI / 5.0;
                    let x = config.cx + config.size * angle.cos();
                    let y = config.cy - config.size * angle.sin();
                    format!("{} {}", x, y)
                })
                .collect::<Vec<_>>()
                .join(" ");

            writeln!(
                svg,
                r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                points, fill_str, stroke_str, config.stroke_width, config.opacity
            )
        }
        PointShape::Hexagon => {
            let points = (0..6)
                .map(|i| {
                    let angle =
                        std::f64::consts::PI / 2.0 + i as f64 * 2.0 * std::f64::consts::PI / 6.0;
                    let x = config.cx + config.size * angle.cos();
                    let y = config.cy - config.size * angle.sin();
                    format!("{} {}", x, y)
                })
                .collect::<Vec<_>>()
                .join(" ");

            writeln!(
                svg,
                r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                points, fill_str, stroke_str, config.stroke_width, config.opacity
            )
        }
        PointShape::Octagon => {
            let points = (0..8)
                .map(|i| {
                    let angle =
                        std::f64::consts::PI / 8.0 + i as f64 * 2.0 * std::f64::consts::PI / 8.0;
                    let x = config.cx + config.size * angle.cos();
                    let y = config.cy - config.size * angle.sin();
                    format!("{} {}", x, y)
                })
                .collect::<Vec<_>>()
                .join(" ");

            writeln!(
                svg,
                r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                points, fill_str, stroke_str, config.stroke_width, config.opacity
            )
        }
        PointShape::Star => {
            let outer_size = config.size;
            let inner_size = config.size * 0.5;
            let points = (0..10)
                .map(|i| {
                    let radius = if i % 2 == 0 { outer_size } else { inner_size };
                    let angle = std::f64::consts::PI / 2.0 + i as f64 * std::f64::consts::PI / 5.0;
                    let x = config.cx + radius * angle.cos();
                    let y = config.cy - radius * angle.sin();
                    format!("{} {}", x, y)
                })
                .collect::<Vec<_>>()
                .join(" ");

            writeln!(
                svg,
                r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                points, fill_str, stroke_str, config.stroke_width, config.opacity
            )
        }
    }
}
