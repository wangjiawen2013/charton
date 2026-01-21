use crate::core::context::SharedRenderingContext;
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/// Orchestrates the visual rendering of both horizontal and vertical axes.
/// 
/// This manager decouples data scales from physical positions. By rendering based 
/// on physical orientation (Bottom/Left) rather than logical axes (X/Y), it ensures 
/// that the chart frame remains stable even when coordinates are flipped.
pub fn render_axes(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    x_label: &str,
    y_label: &str,
) -> Result<(), ChartonError> {
    // Resolve which data label belongs to which physical position.
    // If flipped, the Y-scale data is projected onto the visual Bottom axis.
    let (bottom_label, left_label) = if ctx.coord.is_flipped() {
        (y_label, x_label)
    } else {
        (x_label, y_label)
    };

    // 1. Process the Physical Bottom Axis
    draw_axis_line(svg, theme, ctx, true)?;
    draw_ticks_and_labels(svg, theme, ctx, true)?;
    draw_axis_title(svg, theme, ctx, bottom_label, true)?;

    // 2. Process the Physical Left Axis
    draw_axis_line(svg, theme, ctx, false)?;
    draw_ticks_and_labels(svg, theme, ctx, false)?;
    draw_axis_title(svg, theme, ctx, left_label, false)?;

    Ok(())
}

/// Renders the structural spine of an axis.
///
/// NOTE: We avoid `coord.transform` here because the axis spine is a part of the 
/// static physical frame. We define its position directly using panel boundaries.
fn draw_axis_line(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    is_bottom: bool,
) -> Result<(), ChartonError> {
    let panel = &ctx.panel;

    let (x1, y1, x2, y2) = if is_bottom {
        // Horizontal line at the bottom of the panel
        (panel.x, panel.y + panel.height, panel.x + panel.width, panel.y + panel.height)
    } else {
        // Vertical line at the left of the panel
        (panel.x, panel.y, panel.x, panel.y + panel.height)
    };

    writeln!(
        svg,
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}" stroke-linecap="square"/>"#,
        x1, y1, x2, y2, theme.label_color.as_str(), theme.axis_width
    )?;
    Ok(())
}

/// Renders tick marks and text labels.
///
/// It determines the correct `Scale` based on the flip state and calculates 
/// pixel positions by interpolating within the physical panel dimensions.
fn draw_ticks_and_labels(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    is_bottom: bool,
) -> Result<(), ChartonError> {
    let coord = ctx.coord;
    let panel = &ctx.panel;
    let is_flipped = coord.is_flipped();
    
    // Select the scale based on physical orientation and flip state.
    let target_scale = if is_flipped {
        if is_bottom { coord.get_y_scale() } else { coord.get_x_scale() }
    } else {
        if is_bottom { coord.get_x_scale() } else { coord.get_y_scale() }
    };
    
    let ticks = target_scale.ticks(8); 
    let tick_len = 6.0;
    let angle = if is_bottom { theme.x_tick_label_angle } else { theme.y_tick_label_angle };

    for tick in ticks {
        let norm_pos = target_scale.normalize(tick.value);
        
        // Calculate pixel anchors directly from panel boundaries to bypass logic swapping.
        let (px, py) = if is_bottom {
            (panel.x + norm_pos * panel.width, panel.y + panel.height)
        } else {
            // SVG Y-axis is inverted: 1.0 (top) is panel.y, 0.0 (bottom) is panel.y + height.
            (panel.x, panel.y + (1.0 - norm_pos) * panel.height)
        };

        let (x2, y2, dx, dy, anchor, baseline) = if is_bottom {
            let x_anchor = if angle == 0.0 { "middle" } else { "end" };
            (px, py + tick_len, 0.0, tick_len + theme.tick_label_padding, x_anchor, "hanging")
        } else {
            (px - tick_len, py, -(tick_len + theme.tick_label_padding + 1.0), 0.0, "end", "central")
        };
        
        // Draw Tick Line
        writeln!(svg, r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}"/>"#,
            px, py, x2, y2, theme.label_color.as_str(), theme.tick_width)?;

        // Draw Label Text
        let final_x = px + dx;
        let final_y = py + dy;
        let transform = if angle != 0.0 {
            format!(r#" transform="rotate({:.1}, {:.2}, {:.2})""#, angle, final_x, final_y)
        } else { 
            "".to_string() 
        };

        writeln!(svg, r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}"{}>{}</text>"#,
            final_x, final_y, theme.tick_label_size, theme.tick_label_family,
            theme.tick_label_color.as_str(), anchor, baseline, transform, tick.label
        )?;
    }
    Ok(())
}

/// Renders the axis title (e.g., "mpg") with collision avoidance.
///
/// It uses trigonometric projection to calculate the offset needed to clear 
/// rotated tick labels, ensuring the title stays within the allocated margins.
fn draw_axis_title(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    label: &str,
    is_bottom: bool,
) -> Result<(), ChartonError> {
    if label.is_empty() { return Ok(()); }
    
    let panel = &ctx.panel;
    let coord = ctx.coord;
    let is_flipped = coord.is_flipped();

    let tick_line_len = 6.0;
    let title_gap = 5.0;

    // Resolve the appropriate angle and scale for footprint measurement.
    let (angle_rad, target_scale) = if is_flipped {
        if is_bottom { (theme.y_tick_label_angle.to_radians(), coord.get_y_scale()) } 
        else { (theme.x_tick_label_angle.to_radians(), coord.get_x_scale()) }
    } else {
        if is_bottom { (theme.x_tick_label_angle.to_radians(), coord.get_x_scale()) } 
        else { (theme.y_tick_label_angle.to_radians(), coord.get_y_scale()) }
    };

    let ticks = target_scale.ticks(8);

    if is_bottom {
        let x = panel.x + panel.width / 2.0;
        let max_tick_height = ticks.iter()
            .map(|t| {
                let w = crate::core::utils::estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;
                // An empirical algorithm to account for the rotation
                if is_flipped {
                    (w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()) + 3.0
                } else {
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
                }
            })
            .fold(0.0, f32::max);

        let v_offset = tick_line_len + max_tick_height + theme.label_padding + title_gap;
        let y = panel.y + panel.height + v_offset; 
        
        writeln!(svg, r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" dominant-baseline="hanging">{}</text>"#,
            x, y, theme.label_size, theme.label_family, theme.label_color.as_str(), label
        )?;
    } else {
        let y = panel.y + panel.height / 2.0;
        let max_tick_width = ticks.iter()
            .map(|t| {
                let w = crate::core::utils::estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;
                // An empirical algorithm to account for the rotation
                if is_flipped {
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs() - 4.0
                } else {
                    w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
                }
            })
            .fold(0.0, f32::max);

        let h_offset = tick_line_len + max_tick_width + theme.label_padding + title_gap + theme.label_size;
        let x = panel.x - h_offset; 
        
        writeln!(svg, r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" transform="rotate(-90, {:.2}, {:.2})" dominant-baseline="middle">{}</text>"#,
            x, y, theme.label_size, theme.label_family, theme.label_color.as_str(), x, y, label
        )?;
    }
    Ok(())
}