use crate::visual::{color::SingleColor, shape::PointShape};
use std::fmt::Write;

/// Renders a point with all its visual properties into the SVG string.
/// This is a simplified version for legend rendering.
///
/// # Parameters
///
/// - `svg`: A mutable reference to a string used to accumulate the generated SVG content.
/// - `cx`: The x-coordinate of the point's center (f64).
/// - `cy`: The y-coordinate of the point's center (f64).
/// - `fill_color`: The fill color scheme.
/// - `shape`: The shape variant of the point.
/// - `size`: The size of the point (radius or side length, depending on the shape).
/// - `opacity`: The opacity level (between 0.0 and 1.0).
/// - `stroke_color`: The stroke color scheme.
/// - `stroke_width`: The stroke width.
///
/// # Returns
///
/// Returns a `std::fmt::Result` indicating success or failure of the write operation.
pub(crate) fn render_point(
    svg: &mut String,
    cx: f64,
    cy: f64,
    fill_color: &Option<SingleColor>,
    shape: &PointShape,
    size: f64,
    opacity: f64,
    stroke_color: &Option<SingleColor>,
    stroke_width: f64,
) -> std::fmt::Result {
    // Convert ColorScheme to string values for SVG
    let fill_str = if let Some(color) = fill_color {
        color.get_color()
    } else {
        "none".to_string()
    };
    let stroke_str = if let Some(color) = stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    match shape {
        // Basic shapes implementation
        PointShape::Circle => {
            writeln!(
                svg,
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                cx, cy, size, fill_str, stroke_str, stroke_width, opacity
            )
        }
        PointShape::Square => {
            writeln!(
                svg,
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                cx - size,
                cy - size,
                size * 2.0,
                size * 2.0,
                fill_str,
                stroke_str,
                stroke_width,
                opacity
            )
        }
        PointShape::Diamond => {
            writeln!(
                svg,
                r#"<polygon points="{} {} {} {} {} {} {} {}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                cx,
                cy - size, // Top
                cx + size,
                cy, // Right
                cx,
                cy + size, // Bottom
                cx - size,
                cy, // Left
                fill_str,
                stroke_str,
                stroke_width,
                opacity
            )
        }
        PointShape::Triangle => {
            let height = size * 1.732; // Height of equilateral triangle
            writeln!(
                svg,
                r#"<polygon points="{} {} {} {} {} {}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                cx,
                cy - height * 2.0 / 3.0, // Top
                cx - size,
                cy + height / 3.0, // Bottom left
                cx + size,
                cy + height / 3.0, // Bottom right
                fill_str,
                stroke_str,
                stroke_width,
                opacity
            )
        }
        PointShape::Pentagon => {
            let points = (0..5)
                .map(|i| {
                    let angle =
                        std::f64::consts::PI / 2.0 + i as f64 * 2.0 * std::f64::consts::PI / 5.0;
                    let x = cx + size * angle.cos();
                    let y = cy - size * angle.sin();
                    format!("{} {}", x, y)
                })
                .collect::<Vec<_>>()
                .join(" ");

            writeln!(
                svg,
                r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                points, fill_str, stroke_str, stroke_width, opacity
            )
        }
        PointShape::Hexagon => {
            let points = (0..6)
                .map(|i| {
                    let angle =
                        std::f64::consts::PI / 2.0 + i as f64 * 2.0 * std::f64::consts::PI / 6.0;
                    let x = cx + size * angle.cos();
                    let y = cy - size * angle.sin();
                    format!("{} {}", x, y)
                })
                .collect::<Vec<_>>()
                .join(" ");

            writeln!(
                svg,
                r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                points, fill_str, stroke_str, stroke_width, opacity
            )
        }
        PointShape::Octagon => {
            let points = (0..8)
                .map(|i| {
                    let angle =
                        std::f64::consts::PI / 8.0 + i as f64 * 2.0 * std::f64::consts::PI / 8.0;
                    let x = cx + size * angle.cos();
                    let y = cy - size * angle.sin();
                    format!("{} {}", x, y)
                })
                .collect::<Vec<_>>()
                .join(" ");

            writeln!(
                svg,
                r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                points, fill_str, stroke_str, stroke_width, opacity
            )
        }
        PointShape::Star => {
            let outer_size = size;
            let inner_size = size * 0.5;
            let points = (0..10)
                .map(|i| {
                    let radius = if i % 2 == 0 { outer_size } else { inner_size };
                    let angle = std::f64::consts::PI / 2.0 + i as f64 * std::f64::consts::PI / 5.0;
                    let x = cx + radius * angle.cos();
                    let y = cy - radius * angle.sin();
                    format!("{} {}", x, y)
                })
                .collect::<Vec<_>>()
                .join(" ");

            writeln!(
                svg,
                r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                points, fill_str, stroke_str, stroke_width, opacity
            )
        }
    }
}
