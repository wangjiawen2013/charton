use crate::visual::color::SingleColor;
use std::fmt::Write;

/// Renders a box plot element (box, whiskers, and outliers) into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `x_center` - X-coordinate of the box center
/// * `min_y` - Y-coordinate of the minimum value
/// * `q1_y` - Y-coordinate of the first quartile
/// * `median_y` - Y-coordinate of the median
/// * `q3_y` - Y-coordinate of the third quartile
/// * `max_y` - Y-coordinate of the maximum value
/// * `box_width` - Width of the box
/// * `fill_color` - Fill color for the box
/// * `stroke_color` - Stroke color for all elements
/// * `stroke_width` - Width of strokes
/// * `opacity` - Opacity level (0.0 to 1.0)
/// * `outliers` - Vector of outlier Y-coordinates
/// * `outlier_color` - Color for outlier points
/// * `outlier_size` - Size of outlier points
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_vertical_box(
    svg: &mut String,
    x_center: f64,
    min_y: f64,
    q1_y: f64,
    median_y: f64,
    q3_y: f64,
    max_y: f64,
    box_width: f64,
    fill_color: &Option<SingleColor>,
    stroke_color: &Option<SingleColor>,
    stroke_width: f64,
    opacity: f64,
    outliers: &[f64],
    outlier_color: &Option<SingleColor>,
    outlier_size: f64,
) -> std::fmt::Result {
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

    let outlier_str = if let Some(color) = outlier_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Calculate box edges based on the center position
    let box_left = x_center - box_width / 2.0;
    let box_right = x_center + box_width / 2.0;

    // Draw vertical line (whiskers)
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        x_center, min_y, x_center, q1_y, stroke_str, stroke_width
    )?;

    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        x_center, q3_y, x_center, max_y, stroke_str, stroke_width
    )?;

    // Draw box (IQR)
    let box_height = (q1_y - q3_y).abs();
    let box_y = q3_y.min(q1_y);

    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" opacity="{}" stroke="{}" stroke-width="{}" />"#,
        box_left, box_y, box_width, box_height, fill_str, opacity, stroke_str, stroke_width
    )?;

    // Draw median line
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        box_left,
        median_y,
        box_right,
        median_y,
        stroke_str,
        stroke_width * 2.0
    )?;

    // Draw outliers
    for &outlier_y in outliers {
        writeln!(
            svg,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" opacity="{}" />"#,
            x_center, outlier_y, outlier_size, outlier_str, opacity
        )?;
    }

    Ok(())
}

/// Renders a horizontal box plot element into the SVG string
///
/// # Parameters
/// * `svg` - A mutable reference to the SVG string being built
/// * `y_center` - Y-coordinate of the box center
/// * `min_x` - X-coordinate of the minimum value
/// * `q1_x` - X-coordinate of the first quartile
/// * `median_x` - X-coordinate of the median
/// * `q3_x` - X-coordinate of the third quartile
/// * `max_x` - X-coordinate of the maximum value
/// * `box_height` - Height of the box
/// * `fill_color` - Fill color for the box
/// * `stroke_color` - Stroke color for all elements
/// * `stroke_width` - Width of strokes
/// * `opacity` - Opacity level (0.0 to 1.0)
/// * `outliers` - Vector of outlier X-coordinates
/// * `outlier_color` - Color for outlier points
/// * `outlier_size` - Size of outlier points
///
/// # Returns
/// Result indicating success or failure of the operation
pub(crate) fn render_horizontal_box(
    svg: &mut String,
    y_center: f64,
    min_x: f64,
    q1_x: f64,
    median_x: f64,
    q3_x: f64,
    max_x: f64,
    box_height: f64,
    fill_color: &Option<SingleColor>,
    stroke_color: &Option<SingleColor>,
    stroke_width: f64,
    opacity: f64,
    outliers: &[f64],
    outlier_color: &Option<SingleColor>,
    outlier_size: f64,
) -> std::fmt::Result {
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

    let outlier_str = if let Some(color) = outlier_color {
        color.get_color()
    } else {
        "none".to_string()
    };

    // Calculate box edges based on the center position
    let box_top = y_center - box_height / 2.0;
    let box_bottom = y_center + box_height / 2.0;

    // Draw horizontal line (whiskers)
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        min_x, y_center, q1_x, y_center, stroke_str, stroke_width
    )?;

    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        q3_x, y_center, max_x, y_center, stroke_str, stroke_width
    )?;

    // Draw box (IQR)
    let box_width = (q3_x - q1_x).abs();
    let box_x = q1_x.min(q3_x);

    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" opacity="{}" stroke="{}" stroke-width="{}" />"#,
        box_x, box_top, box_width, box_height, fill_str, opacity, stroke_str, stroke_width
    )?;

    // Draw median line
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        median_x,
        box_top,
        median_x,
        box_bottom,
        stroke_str,
        stroke_width * 2.0
    )?;

    // Draw outliers
    for &outlier_x in outliers {
        writeln!(
            svg,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" opacity="{}" />"#,
            outlier_x, y_center, outlier_size, outlier_str, opacity
        )?;
    }

    Ok(())
}
