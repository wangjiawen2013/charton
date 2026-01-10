use crate::coordinate::{CoordinateTrait, Rect};
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/* *
 * GUIDES DEFINITION:
 * Guides are visual elements (Axes and Legends) that provide the context 
 * needed to interpret the data. 
 *
 * Architecture:
 * This module queries the Coordinate system to retrieve Scales, 
 * calculates logical tick positions, and projects them into SVG space.
 */

/// Renders all axis components: Lines, Ticks, Labels, and Titles.
pub fn render_axes(
    svg: &mut String,
    theme: &Theme,
    coord: &dyn CoordinateTrait,
    panel: &Rect,
    x_label: &str,
    y_label: &str,
) -> Result<(), ChartonError> {
    // --- 1. Draw Axis Lines ---
    draw_axis_line(svg, theme, coord, panel, true)?;
    draw_axis_line(svg, theme, coord, panel, false)?;

    // --- 2. Draw Ticks and Tick Labels ---
    // Responsibility is now fully inside the function to avoid redundant parameters.
    draw_ticks_and_labels(svg, theme, coord, panel, true)?;
    draw_ticks_and_labels(svg, theme, coord, panel, false)?;

    // --- 3. Draw Axis Titles ---
    draw_axis_title(svg, theme, panel, x_label, true)?;
    draw_axis_title(svg, theme, panel, y_label, false)?;

    Ok(())
}

/// Draws the primary spine of the axis.
fn draw_axis_line(
    svg: &mut String,
    theme: &Theme,
    coord: &dyn CoordinateTrait,
    panel: &Rect,
    is_x: bool,
) -> Result<(), ChartonError> {
    let (p1x, p1y) = coord.transform(0.0, 0.0, panel);
    let (p2x, p2y) = if is_x {
        coord.transform(1.0, 0.0, panel)
    } else {
        coord.transform(0.0, 1.0, panel)
    };

    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
        p1x, p1y, p2x, p2y, theme.label_color, theme.axis_stroke_width
    )?;
    Ok(())
}

/// Draws tick marks and text labels by querying the coordinate system internally.
fn draw_ticks_and_labels(
    svg: &mut String,
    theme: &Theme,
    coord: &dyn CoordinateTrait,
    panel: &Rect,
    is_x: bool,
) -> Result<(), ChartonError> {
    // Get scales from the coord system
    let (x_scale, y_scale) = coord.get_scales();
    
    // Select the target scale and generate logical ticks
    let target_scale = if is_x { x_scale } else { y_scale };
    let ticks = target_scale.ticks(10);

    for tick in ticks {
        // Position transformation: Data Value -> Normalized [0,1] -> Pixel Coordinates
        let (px, py) = if is_x {
            coord.transform(x_scale.normalize(tick.value), 0.0, panel)
        } else {
            coord.transform(0.0, y_scale.normalize(tick.value), panel)
        };

        // Draw the tick mark line
        let tick_len = 5.0;
        let (x2, y2) = if is_x { (px, py + tick_len) } else { (px - tick_len, py) };
        
        writeln!(
            svg, 
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
            px, py, x2, y2, theme.label_color, theme.tick_stroke_width
        )?;

        // Draw the tick label text
        let anchor = if is_x { "middle" } else { "end" };
        let (dx, dy) = if is_x { 
            (0.0, theme.tick_label_font_size as f64 + 2.0) 
        } else { 
            (-8.0, 4.0) 
        };

        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}">{}</text>"#,
            px + dx, py + dy, theme.tick_label_font_size, theme.tick_label_font_family,
            theme.label_color, anchor, if is_x { "auto" } else { "middle" }, tick.label
        )?;
    }
    Ok(())
}

/// Draws the main descriptive title of the axis.
fn draw_axis_title(
    svg: &mut String,
    theme: &Theme,
    panel: &Rect,
    label: &str,
    is_x: bool,
) -> Result<(), ChartonError> {
    if label.is_empty() { return Ok(()); }

    if is_x {
        let x = panel.x + panel.width / 2.0;
        let y = panel.y + panel.height + 45.0; 
        
        writeln!(
            svg, 
            r#"<text x="{}" y="{}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, label
        )?;
    } else {
        let x = panel.x - 50.0; 
        let y = panel.y + panel.height / 2.0;
        
        writeln!(
            svg, 
            r#"<text x="{}" y="{}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" transform="rotate(-90, {}, {})">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, x, y, label
        )?;
    }
    Ok(())
}