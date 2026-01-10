use crate::coordinate::{CoordinateTrait, Rect};
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/// Renders all axis components: Lines, Ticks, Labels, and Titles.
pub fn render_axes(
    svg: &mut String,
    theme: &Theme,
    coord: &dyn CoordinateTrait,
    panel: &Rect,
    x_label: &str,
    y_label: &str,
) -> Result<(), ChartonError> {
    // 1. Draw Axis Spines (Main lines)
    draw_axis_line(svg, theme, coord, panel, true)?;
    draw_axis_line(svg, theme, coord, panel, false)?;

    // 2. Draw Ticks and Tick Labels
    draw_ticks_and_labels(svg, theme, coord, panel, true)?;
    draw_ticks_and_labels(svg, theme, coord, panel, false)?;

    // 3. Draw Axis Titles
    draw_axis_title(svg, theme, panel, x_label, true)?;
    draw_axis_title(svg, theme, panel, y_label, false)?;

    Ok(())
}

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
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}" stroke-linecap="square"/>"#,
        p1x, p1y, p2x, p2y, theme.label_color, theme.axis_stroke_width
    )?;
    Ok(())
}

fn draw_ticks_and_labels(
    svg: &mut String,
    theme: &Theme,
    coord: &dyn CoordinateTrait,
    panel: &Rect,
    is_x: bool,
) -> Result<(), ChartonError> {
    let (x_scale, y_scale) = coord.get_scales();
    let target_scale = if is_x { x_scale } else { y_scale };
    
    // Generate ticks (e.g., 8 is a standard default for readability)
    let ticks = target_scale.ticks(8); 

    for tick in ticks {
        let norm_pos = target_scale.normalize(tick.value);

        let (px, py) = if is_x {
            coord.transform(norm_pos, 0.0, panel)
        } else {
            coord.transform(0.0, norm_pos, panel)
        };

        // --- Draw Tick Mark ---
        let tick_len = 6.0;
        let (x2, y2) = if is_x { (px, py + tick_len) } else { (px - tick_len, py) };
        
        writeln!(
            svg, 
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}"/>"#,
            px, py, x2, y2, theme.label_color, theme.tick_stroke_width
        )?;

        // --- Draw Tick Label ---
        let anchor = if is_x { "middle" } else { "end" };
        
        // Calculate dynamic offset using theme's tick_label_padding
        let (dx, dy) = if is_x { 
            (0.0, theme.tick_label_font_size as f64 + theme.tick_label_padding) 
        } else { 
            (-(theme.tick_label_padding + 2.0), theme.tick_label_font_size as f64 * 0.35) 
        };

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}">{}</text>"#,
            px + dx, py + dy, theme.tick_label_font_size, theme.tick_label_font_family,
            theme.tick_label_color, anchor, if is_x { "hanging" } else { "middle" }, tick.label
        )?;
    }
    Ok(())
}

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
        // Use theme.x_label_padding for vertical distance from the panel bottom
        let y = panel.y + panel.height + theme.x_label_padding + 25.0; 
        
        writeln!(
            svg, 
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, label
        )?;
    } else {
        // Use theme.y_label_padding for horizontal distance from the panel left
        let x = panel.x - (theme.y_label_padding + 35.0); 
        let y = panel.y + panel.height / 2.0;
        
        writeln!(
            svg, 
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" transform="rotate(-90, {:.2}, {:.2})">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, x, y, label
        )?;
    }
    Ok(())
}