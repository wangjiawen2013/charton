use crate::core::context::SharedRenderingContext;
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/// Orchestrates the visual rendering of both horizontal and vertical axes.
/// 
/// This function acts as the high-level manager for axis manifestation. It 
/// handles the logical-to-physical mapping required by the "Coordinate Flip" 
/// feature. In the Grammar of Graphics, while the data scales remain fixed, 
/// their visual projection can be swapped (e.g., a Bar chart becoming a 
/// Horizontal Bar chart).
pub fn render_axes(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    x_label: &str,
    y_label: &str,
) -> Result<(), ChartonError> {
    // Determine the mapping between data labels and visual positions.
    // When the coordinate system is flipped, the data's Y-axis is projected
    // onto the visual horizontal axis (the bottom of the panel).
    let (bottom_label, left_label) = if ctx.coord.is_flipped() {
        (y_label, x_label)
    } else {
        (x_label, y_label)
    };

    // 1. Process the Horizontal Axis (Visual Bottom)
    // Regardless of whether it represents X or Y data, it is rendered at the bottom.
    draw_axis_line(svg, theme, ctx, true)?;
    draw_ticks_and_labels(svg, theme, ctx, true)?;
    draw_axis_title(svg, theme, ctx, bottom_label, true)?;

    // 2. Process the Vertical Axis (Visual Left)
    // Regardless of whether it represents X or Y data, it is rendered on the left.
    draw_axis_line(svg, theme, ctx, false)?;
    draw_ticks_and_labels(svg, theme, ctx, false)?;
    draw_axis_title(svg, theme, ctx, left_label, false)?;

    Ok(())
}

/// Renders the main structural line (spine) of an axis.
///
/// It utilizes the coordinate system's transformation matrix to find 
/// the pixel coordinates of the [0.0, 1.0] normalized range.
fn draw_axis_line(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    is_visual_x: bool,
) -> Result<(), ChartonError> {
    let coord = ctx.coord;
    let panel = &ctx.panel;

    // The origin (0.0, 0.0) in normalized space is the anchor for both axes.
    let (p1x, p1y) = coord.transform(0.0, 0.0, panel);
    
    // (1,0) denotes the end of the horizontal axis; (0,1) for the vertical.
    let (p2x, p2y) = if is_visual_x {
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

/// Renders the tick marks and text labels along an axis.
///
/// This function calculates the appropriate scale to use based on the `is_visual_x`
/// intent and the `flipped` state of the coordinate system. It ensures that 
/// ticks always point away from the data panel to avoid visual clutter.
fn draw_ticks_and_labels(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    is_visual_x: bool,
) -> Result<(), ChartonError> {
    let coord = ctx.coord;
    let panel = &ctx.panel;
    let is_flipped = coord.is_flipped();
    
    // Resolve which data scale (X or Y) is currently mapped to this visual axis.
    let target_scale = if is_flipped {
        if is_visual_x { coord.get_y_scale() } else { coord.get_x_scale() }
    } else {
        if is_visual_x { coord.get_x_scale() } else { coord.get_y_scale() }
    };
    
    // Request a set of 'pretty' tick values from the scale (defaulting to ~8).
    let ticks = target_scale.ticks(8); 
    let tick_len = 6.0;
    let angle = if is_visual_x { theme.x_tick_label_angle } else { theme.y_tick_label_angle };

    for tick in ticks {
        let norm_pos = target_scale.normalize(tick.value);
        
        // Map the normalized position back to absolute screen pixels.
        let (px, py) = if is_visual_x {
            coord.transform(norm_pos, 0.0, panel)
        } else {
            coord.transform(0.0, norm_pos, panel)
        };

        // Calculate geometric offsets for the tick lines and text anchors.
        // If flipped, 'Visual X' is physically on the panel's left side.
        // Otherwise, it is physically on the panel's bottom side.
        let (x2, y2, dx, dy, anchor, baseline) = if is_flipped {
            if is_visual_x {
                // Physical Left (Representing Logical Y)
                (px - tick_len, py, -(tick_len + theme.tick_label_padding + 1.0), 0.0, "end", "central")
            } else {
                // Physical Bottom (Representing Logical X)
                (px, py + tick_len, 0.0, tick_len + theme.tick_label_padding, "middle", "hanging")
            }
        } else {
            if is_visual_x {
                // Physical Bottom (Representing Logical X)
                let x_anchor = if angle == 0.0 { "middle" } else { "end" };
                (px, py + tick_len, 0.0, tick_len + theme.tick_label_padding, x_anchor, "hanging")
            } else {
                // Physical Left (Representing Logical Y)
                (px - tick_len, py, -(tick_len + theme.tick_label_padding + 1.0), 0.0, "end", "central")
            }
        };
        
        // 1. Draw the tick mark line
        writeln!(svg, r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}"/>"#,
            px, py, x2, y2, theme.label_color, theme.tick_stroke_width)?;

        // 2. Draw the tick label with optional rotation
        let final_x = px + dx;
        let final_y = py + dy;
        let transform = if angle != 0.0 {
            format!(r#" transform="rotate({:.1}, {:.2}, {:.2})""#, angle, final_x, final_y)
        } else { "".to_string() };

        writeln!(svg, r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}"{}>{}</text>"#,
            final_x, final_y, theme.tick_label_font_size, theme.tick_label_font_family,
            theme.tick_label_color, anchor, baseline, transform, tick.label
        )?;
    }
    Ok(())
}

/// Draws the axis title (e.g., "Weight (kg)") while avoiding collisions with tick labels.
///
/// This function dynamically calculates the title's position by measuring the 
/// maximum footprint (width or height) of all rendered tick labels. It uses 
/// trigonometric projections to account for label rotation.
fn draw_axis_title(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    label: &str,
    is_visual_x: bool,
) -> Result<(), ChartonError> {
    if label.is_empty() { return Ok(()); }
    
    let panel = &ctx.panel;
    let coord = ctx.coord;
    let is_flipped = coord.is_flipped();

    let tick_line_len = 6.0;
    let safety_buffer = 5.0;

    // Resolve the physical orientation to decide which side of the panel to draw on.
    let (is_physically_bottom, angle_rad, target_scale, label_padding) = if is_flipped {
        if is_visual_x {
            (false, theme.x_tick_label_angle.to_radians(), coord.get_y_scale(), theme.x_label_padding)
        } else {
            (true, theme.y_tick_label_angle.to_radians(), coord.get_x_scale(), theme.y_label_padding)
        }
    } else {
        if is_visual_x {
            (true, theme.x_tick_label_angle.to_radians(), coord.get_x_scale(), theme.x_label_padding)
        } else {
            (false, theme.y_tick_label_angle.to_radians(), coord.get_y_scale(), theme.y_label_padding)
        }
    };

    let ticks = target_scale.ticks(8);

    if is_physically_bottom {
        // --- Render title for the bottom-aligned axis ---
        let x = panel.x + panel.width / 2.0;
        
        // Calculate the vertical "footprint" of tick labels to push the title further down.
        let max_height = ticks.iter()
            .map(|t| {
                let w = crate::core::layout::estimate_text_width(&t.label, theme.tick_label_font_size);
                let h = theme.tick_label_font_size;
                w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
            })
            .fold(0.0, f64::max);

        let v_offset = tick_line_len + max_height + safety_buffer + label_padding;
        let y = panel.y + panel.height + v_offset; 
        
        writeln!(svg, r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" dominant-baseline="hanging">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, label)?;
    } else {
        // --- Render title for the left-aligned axis ---
        
        // Calculate the horizontal "footprint" of tick labels to push the title further left.
        let max_width = ticks.iter()
            .map(|t| {
                let w = crate::core::layout::estimate_text_width(&t.label, theme.tick_label_font_size);
                let h = theme.tick_label_font_size;
                w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
            })
            .fold(0.0, f64::max);

        let h_offset = tick_line_len + max_width + safety_buffer + label_padding;
        let x = panel.x - h_offset; 
        let y = panel.y + panel.height / 2.0;
        
        // Vertical titles are rotated -90 degrees around their own center.
        writeln!(svg, r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" transform="rotate(-90, {:.2}, {:.2})" dominant-baseline="auto">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, x, y, label)?;
    }
    Ok(())
}