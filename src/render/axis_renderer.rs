use crate::core::context::PanelContext;
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/// Orchestrates the visual rendering of both horizontal and vertical axes for a panel.
/// 
/// This function is "Panel-aware": it renders axes relative to the `Rect` provided 
/// in the `PanelContext`. In a faceted chart, this is called for each individual panel.
pub fn render_axes(
    svg: &mut String,
    theme: &Theme,
    ctx: &PanelContext,
    x_label: &str,
    y_label: &str,
) -> Result<(), ChartonError> {
    // Determine which data label belongs to which physical position based on flip state.
    // If flipped, the Y-scale data is projected onto the visual Bottom axis.
    let (bottom_label, left_label) = if ctx.coord.is_flipped() {
        (y_label, x_label)
    } else {
        (x_label, y_label)
    };

    // 1. Process the Physical Bottom Axis (X-axis in standard, Y-axis in flipped)
    draw_axis_line(svg, theme, ctx, true)?;
    draw_ticks_and_labels(svg, theme, ctx, true)?;
    draw_axis_title(svg, theme, ctx, bottom_label, true)?;

    // 2. Process the Physical Left Axis (Y-axis in standard, X-axis in flipped)
    draw_axis_line(svg, theme, ctx, false)?;
    draw_ticks_and_labels(svg, theme, ctx, false)?;
    draw_axis_title(svg, theme, ctx, left_label, false)?;

    Ok(())
}

/// Renders the straight line (spine) of the axis.
fn draw_axis_line(
    svg: &mut String,
    theme: &Theme,
    ctx: &PanelContext,
    is_bottom: bool,
) -> Result<(), ChartonError> {
    let panel = &ctx.panel;

    let (x1, y1, x2, y2) = if is_bottom {
        // Horizontal line at the bottom edge of the panel
        (panel.x, panel.y + panel.height, panel.x + panel.width, panel.y + panel.height)
    } else {
        // Vertical line at the left edge of the panel
        (panel.x, panel.y, panel.x, panel.y + panel.height)
    };

    writeln!(
        svg,
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}" stroke-linecap="square"/>"#,
        x1, y1, x2, y2, theme.label_color.to_css_string(), theme.axis_width
    )?;
    Ok(())
}

/// Renders the tick marks and their corresponding text labels.
fn draw_ticks_and_labels(
    svg: &mut String,
    theme: &Theme,
    ctx: &PanelContext,
    is_bottom: bool,
) -> Result<(), ChartonError> {
    let coord = &ctx.coord;
    let panel = &ctx.panel;
    let is_flipped = coord.is_flipped();
    
    // Select the correct scale object based on orientation and flip state.
    let target_scale = if is_flipped {
        if is_bottom { coord.get_y_scale() } else { coord.get_x_scale() }
    } else {
        if is_bottom { coord.get_x_scale() } else { coord.get_y_scale() }
    };

    // --- Adaptive Tick Decimation Strategy ---
    // Define a minimum physical distance (in pixels) between tick marks to ensure 
    // visual clarity and prevent label overlap.
    let pixel_step = 50.0; 
    let available_space = if is_bottom { panel.width } else { panel.height };
    
    // Calculate the ideal number of ticks based on available screen real estate.
    // We ensure at least 2 ticks (start and end) are always requested.
    let suggested_count = (available_space / pixel_step).floor() as usize;
    let final_count = suggested_count.max(2);

    // Delegate the mathematical selection of "pretty" values to the specific Scale implementation.
    let ticks = target_scale.ticks(final_count); 

    let tick_len = 6.0;
    let angle = if is_bottom { theme.x_tick_label_angle } else { theme.y_tick_label_angle };

    for tick in ticks {
        let norm_pos = target_scale.normalize(tick.value);
        
        // Calculate the anchor point on the axis line.
        let (px, py) = if is_bottom {
            (panel.x + norm_pos * panel.width, panel.y + panel.height)
        } else {
            // SVG Y-coordinates increase downwards, so 0.0 (bottom) is at y + height.
            (panel.x, panel.y + (1.0 - norm_pos) * panel.height)
        };

        // Determine text alignment and tick direction.
        let (x2, y2, dx, dy, anchor, baseline) = if is_bottom {
            let x_anchor = if angle == 0.0 { "middle" } else { "end" };
            (px, py + tick_len, 0.0, tick_len + theme.tick_label_padding, x_anchor, "hanging")
        } else {
            (px - tick_len, py, -(tick_len + theme.tick_label_padding + 1.0), 0.0, "end", "central")
        };
        
        // Render the Tick Line
        writeln!(svg, r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}"/>"#,
            px, py, x2, y2, theme.label_color.to_css_string(), theme.tick_width)?;

        // Render the Label Text
        let final_x = px + dx;
        let final_y = py + dy;
        let transform = if angle != 0.0 {
            format!(r#" transform="rotate({:.1}, {:.2}, {:.2})""#, angle, final_x, final_y)
        } else { 
            "".to_string() 
        };

        writeln!(svg, r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}"{}>{}</text>"#,
            final_x, final_y, theme.tick_label_size, theme.tick_label_family,
            theme.tick_label_color.to_css_string(), anchor, baseline, transform, tick.label
        )?;
    }
    Ok(())
}

/// Renders the axis title, calculating offsets to avoid overlapping with tick labels.
fn draw_axis_title(
    svg: &mut String,
    theme: &Theme,
    ctx: &PanelContext,
    label: &str,
    is_bottom: bool,
) -> Result<(), ChartonError> {
    if label.is_empty() { return Ok(()); }
    
    let panel = &ctx.panel;
    let coord = &ctx.coord;
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
                // Empirical layout calculation for rotated text footprint.
                if is_flipped {
                    (w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()) + 3.0
                } else {
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
                }
            })
            .fold(0.0, f64::max);

        let v_offset = tick_line_len + max_tick_height + theme.label_padding + title_gap;
        let y = panel.y + panel.height + v_offset; 
        
        writeln!(svg, r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" dominant-baseline="hanging">{}</text>"#,
            x, y, theme.label_size, theme.label_family, theme.label_color.to_css_string(), label
        )?;
    } else {
        let y = panel.y + panel.height / 2.0;
        let max_tick_width = ticks.iter()
            .map(|t| {
                let w = crate::core::utils::estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;
                if is_flipped {
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs() - 4.0
                } else {
                    w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
                }
            })
            .fold(0.0, f64::max);

        let h_offset = tick_line_len + max_tick_width + theme.label_padding + title_gap + theme.label_size;
        let x = panel.x - h_offset; 
        
        writeln!(svg, r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" transform="rotate(-90, {:.2}, {:.2})" dominant-baseline="middle">{}</text>"#,
            x, y, theme.label_size, theme.label_family, theme.label_color.to_css_string(), x, y, label
        )?;
    }
    Ok(())
}