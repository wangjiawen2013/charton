use crate::error::ChartonError;
use std::fmt::Write;

// Render a single rectangle
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_rect(
    svg: &mut String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: &str,
    opacity: f64,
    stroke: &str,
    stroke_width: f64,
) -> Result<(), ChartonError> {
    write!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" opacity="{}" stroke="{}" stroke-width="{}" />"#,
        x, y, width, height, fill, opacity, stroke, stroke_width
    )?;
    Ok(())
}
