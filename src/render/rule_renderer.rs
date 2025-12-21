use crate::visual::color::SingleColor;
use std::fmt::Write;

pub(crate) struct VerticalRuleConfig {
    pub x: f64,
    pub y1: f64,
    pub y2: f64,
    pub stroke_color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
}

pub(crate) struct HorizontalRuleConfig {
    pub x1: f64,
    pub x2: f64,
    pub y: f64,
    pub stroke_color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
}

/// Renders a vertical rule (line) into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `config` - Configuration parameters for the vertical rule
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_vertical_rule(
    svg: &mut String,
    config: VerticalRuleConfig,
) -> std::fmt::Result {
    // Convert ColorScheme to string values for SVG
    let stroke_str = if let Some(color) = &config.stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
        config.x, config.y1, config.x, config.y2, stroke_str, config.stroke_width, config.opacity
    )
}

/// Renders a horizontal rule (line) into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `config` - Configuration parameters for the horizontal rule
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_horizontal_rule(
    svg: &mut String,
    config: HorizontalRuleConfig,
) -> std::fmt::Result {
    // Convert ColorScheme to string values for SVG
    let stroke_str = if let Some(color) = &config.stroke_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
        config.x1, config.y, config.x2, config.y, stroke_str, config.stroke_width, config.opacity
    )
}
