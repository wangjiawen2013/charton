use crate::visual::color::SingleColor;
use std::fmt::Write;

pub(crate) struct VerticalBoxConfig {
    pub x_center: f64,
    pub min_y: f64,
    pub q1_y: f64,
    pub median_y: f64,
    pub q3_y: f64,
    pub max_y: f64,
    pub box_width: f64,
    pub fill_color: Option<SingleColor>,
    pub stroke_color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
    pub outliers: Vec<f64>,
    pub outlier_color: Option<SingleColor>,
    pub outlier_size: f64,
}

pub(crate) struct HorizontalBoxConfig {
    pub y_center: f64,
    pub min_x: f64,
    pub q1_x: f64,
    pub median_x: f64,
    pub q3_x: f64,
    pub max_x: f64,
    pub box_height: f64,
    pub fill_color: Option<SingleColor>,
    pub stroke_color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
    pub outliers: Vec<f64>,
    pub outlier_color: Option<SingleColor>,
    pub outlier_size: f64,
}

/// Renders a box plot element (box, whiskers, and outliers) into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `config` - Configuration parameters for the vertical box
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_vertical_box(svg: &mut String, config: VerticalBoxConfig) -> std::fmt::Result {
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

    let outlier_str = if let Some(color) = &config.outlier_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Calculate box edges based on the center position
    let box_left = config.x_center - config.box_width / 2.0;
    let box_right = config.x_center + config.box_width / 2.0;

    // Draw vertical line (whiskers)
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        config.x_center,
        config.min_y,
        config.x_center,
        config.q1_y,
        stroke_str,
        config.stroke_width
    )?;

    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        config.x_center,
        config.q3_y,
        config.x_center,
        config.max_y,
        stroke_str,
        config.stroke_width
    )?;

    // Draw box (IQR)
    let box_height = (config.q1_y - config.q3_y).abs();
    let box_y = config.q3_y.min(config.q1_y);

    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" opacity="{}" stroke="{}" stroke-width="{}" />"#,
        box_left,
        box_y,
        config.box_width,
        box_height,
        fill_str,
        config.opacity,
        stroke_str,
        config.stroke_width
    )?;

    // Draw median line
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        box_left,
        config.median_y,
        box_right,
        config.median_y,
        stroke_str,
        config.stroke_width * 2.0
    )?;

    // Draw outliers
    for &outlier_y in &config.outliers {
        writeln!(
            svg,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" opacity="{}" />"#,
            config.x_center, outlier_y, config.outlier_size, outlier_str, config.opacity
        )?;
    }

    Ok(())
}

/// Renders a horizontal box plot element into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `config` - Configuration parameters for the horizontal box
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_horizontal_box(
    svg: &mut String,
    config: HorizontalBoxConfig,
) -> std::fmt::Result {
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

    let outlier_str = if let Some(color) = &config.outlier_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Calculate box edges based on the center position
    let box_top = config.y_center - config.box_height / 2.0;
    let box_bottom = config.y_center + config.box_height / 2.0;

    // Draw horizontal line (whiskers)
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        config.min_x,
        config.y_center,
        config.q1_x,
        config.y_center,
        stroke_str,
        config.stroke_width
    )?;

    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        config.q3_x,
        config.y_center,
        config.max_x,
        config.y_center,
        stroke_str,
        config.stroke_width
    )?;

    // Draw box (IQR)
    let box_width = (config.q3_x - config.q1_x).abs();
    let box_x = config.q1_x.min(config.q3_x);

    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" opacity="{}" stroke="{}" stroke-width="{}" />"#,
        box_x,
        box_top,
        box_width,
        config.box_height,
        fill_str,
        config.opacity,
        stroke_str,
        config.stroke_width
    )?;

    // Draw median line
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        config.median_x,
        box_top,
        config.median_x,
        box_bottom,
        stroke_str,
        config.stroke_width * 2.0
    )?;

    // Draw outliers
    for &outlier_x in &config.outliers {
        writeln!(
            svg,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" opacity="{}" />"#,
            outlier_x, config.y_center, config.outlier_size, outlier_str, config.opacity
        )?;
    }

    Ok(())
}
