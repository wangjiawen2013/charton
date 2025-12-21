use crate::error::ChartonError;
use std::fmt::Write;

pub(crate) struct RectConfig {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub fill: String,
    pub opacity: f64,
    pub stroke: String,
    pub stroke_width: f64,
}

// Render a single rectangle
pub(crate) fn render_rect(svg: &mut String, config: RectConfig) -> Result<(), ChartonError> {
    write!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" opacity="{}" stroke="{}" stroke-width="{}" />"#,
        config.x,
        config.y,
        config.width,
        config.height,
        config.fill,
        config.opacity,
        config.stroke,
        config.stroke_width
    )?;
    Ok(())
}
