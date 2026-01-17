use crate::core::context::SharedRenderingContext;
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/// `AxisRenderer` handles the visual manifestation of the coordinate system.
/// 
/// It translates abstract data scales into tangible SVG elements (lines and text).
/// This renderer is "Flip-Aware": it correctly identifies which scale (X or Y) 
/// should be rendered on the visual bottom vs. visual left based on the 
/// chart's `flipped` configuration.
pub fn render_axes(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    x_label: &str,
    y_label: &str,
) -> Result<(), ChartonError> {
    // Determine the correct labels based on the flip state.
    // If flipped, the data's Y-axis is displayed on the visual X-axis (bottom).
    let (bottom_label, left_label) = if ctx.coord.is_flipped() {
        (y_label, x_label)
    } else {
        (x_label, y_label)
    };

    // 1. Render Visual X-Axis (The Horizontal Bottom Axis)
    draw_axis_line(svg, theme, ctx, true)?;
    draw_ticks_and_labels(svg, theme, ctx, true)?;
    draw_axis_title(svg, theme, ctx, bottom_label, true)?;

    // 2. Render Visual Y-Axis (The Vertical Left Axis)
    draw_axis_line(svg, theme, ctx, false)?;
    draw_ticks_and_labels(svg, theme, ctx, false)?;
    draw_axis_title(svg, theme, ctx, left_label, false)?;

    Ok(())
}

/// Draws the main axis spine (the line representing the axis itself).
///
/// # Arguments
/// * `svg` - The mutable string buffer to append SVG elements to.
/// * `theme` - Visual configuration including colors and stroke widths.
/// * `ctx` - The shared context containing coordinate system and panel layout.
/// * `is_visual_x` - Directional flag: 
///     - `true`: Renders the horizontal axis (Visual Bottom).
///     - `false`: Renders the vertical axis (Visual Left).
///
/// # Flip-Aware Logic
/// This function relies on `coord.transform` to abstract away physical pixels. 
/// In a "flipped" chart, the horizontal axis (`is_visual_x: true`) will 
/// automatically represent the data's Y-scale.
fn draw_axis_line(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    is_visual_x: bool,
) -> Result<(), ChartonError> {
    let coord = ctx.coord;
    let panel = &ctx.panel;

    // The origin (0.0, 0.0) in normalized space always represents 
    // the intersection of the two axes (usually the bottom-left corner).
    let (p1x, p1y) = coord.transform(0.0, 0.0, panel);
    
    // Calculate the end point of the spine using normalized coordinates.
    // (1.0, 0.0) represents the full extent of the primary dimension,
    // and (0.0, 1.0) represents the full extent of the secondary dimension.
    let (p2x, p2y) = if is_visual_x {
        // Full span across the horizontal visual axis.
        coord.transform(1.0, 0.0, panel)
    } else {
        // Full span across the vertical visual axis.
        coord.transform(0.0, 1.0, panel)
    };

    writeln!(
        svg,
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}" stroke-linecap="square"/>"#,
        p1x, p1y, p2x, p2y, theme.label_color, theme.axis_stroke_width
    )?;
    Ok(())
}

/// Renders tick marks and their corresponding text labels.
///
/// This function is "Orientation-Aware": it detects the physical placement of the axis
/// (bottom vs. left) by combining the `is_visual_x` intent with the coordinate system's
/// `flipped` state. It ensures that ticks always point outward from the plot panel
/// and labels are positioned with correct alignment and rotation.
fn draw_ticks_and_labels(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    is_visual_x: bool,
) -> Result<(), ChartonError> {
    let coord = ctx.coord;
    let panel = &ctx.panel;
    let is_flipped = coord.is_flipped();
    
    // Select the appropriate scale based on the visual axis and flip state.
    // In a flipped chart, the visual Bottom Axis represents the data's Y-scale.
    let target_scale = if is_flipped {
        if is_visual_x { coord.get_y_scale() } else { coord.get_x_scale() }
    } else {
        if is_visual_x { coord.get_x_scale() } else { coord.get_y_scale() }
    };
    
    let ticks = target_scale.ticks(8); 
    let tick_len = 6.0;
    let angle = if is_visual_x { theme.x_tick_label_angle } else { theme.y_tick_label_angle };

    for tick in ticks {
        let norm_pos = target_scale.normalize(tick.value);

        // Calculate the base point (px, py) on the axis spine.
        // We use normalized coordinates (pos, 0.0) for the visual Bottom and (0.0, pos) for the visual Left.
        // The coordinate system's transform() handles the internal mapping to screen pixels.
        let (px, py) = if is_visual_x {
            coord.transform(norm_pos, 0.0, panel)
        } else {
            coord.transform(0.0, norm_pos, panel)
        };

        // Determine the physical direction of the tick and label.
        // If flipped, the visual X-axis (is_visual_x: true) is actually rendered 
        // on the physical Left of the panel.
        let (x2, y2, dx, dy, anchor, baseline) = if is_flipped {
            if is_visual_x {
                // Flipped Case: Visual X is physically on the LEFT.
                // Ticks extend to the left (-X), labels are end-anchored.
                (px - tick_len, py, -(tick_len + theme.tick_label_padding + 1.0), 0.0, "end", "central")
            } else {
                // Flipped Case: Visual Y is physically on the BOTTOM.
                // Ticks extend downward (+Y), labels are middle-anchored.
                (px, py + tick_len, 0.0, tick_len + theme.tick_label_padding, "middle", "hanging")
            }
        } else {
            // Standard Case.
            if is_visual_x {
                // Physically on the BOTTOM.
                let x_anchor = if angle == 0.0 { "middle" } else { "end" };
                (px, py + tick_len, 0.0, tick_len + theme.tick_label_padding, x_anchor, "hanging")
            } else {
                // Physically on the LEFT.
                (px - tick_len, py, -(tick_len + theme.tick_label_padding + 1.0), 0.0, "end", "central")
            }
        };
        
        // Render the tick mark line.
        writeln!(
            svg, 
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}"/>"#,
            px, py, x2, y2, theme.label_color, theme.tick_stroke_width
        )?;

        // Position the text label relative to the tick end.
        let final_x = px + dx;
        let final_y = py + dy;
        
        // Apply rotation if specified. The pivot is the calculated label anchor point.
        let transform = if angle != 0.0 {
            format!(r#" transform="rotate({:.1}, {:.2}, {:.2})""#, angle, final_x, final_y)
        } else { 
            "".to_string() 
        };

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}"{}>{}</text>"#,
            final_x, final_y, theme.tick_label_font_size, theme.tick_label_font_family,
            theme.tick_label_color, anchor, baseline, transform, tick.label
        )?;
    }
    Ok(())
}

/// Draws the axis title (X or Y label) with collision avoidance for tick labels.
///
/// This function calculates the optimal placement for the title by measuring 
/// the vertical or horizontal footprint of the tick labels. It accounts for 
/// label rotation using trigonometric projection.
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

    // Map logical axis intent to physical screen location.
    // If the chart is flipped, the logical X-axis title moves to the physical Left.
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

    if is_physically_bottom {
        // --- PHYSICAL BOTTOM RENDERER ---
        // Calculate the midpoint of the panel width.
        let x = panel.x + panel.width / 2.0;
        let ticks = target_scale.ticks(8);

        // Compute the maximum vertical height of rotated tick labels.
        let max_height = ticks.iter()
            .map(|t| {
                let w = crate::core::layout::estimate_text_width(&t.label, theme.tick_label_font_size);
                let h = theme.tick_label_font_size;
                w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
            })
            .fold(0.0, f64::max);

        // Total offset includes tick length, max label height, paddings, and half of the title font size.
        let v_offset = tick_line_len + max_height + safety_buffer + label_padding + (theme.label_font_size / 2.0);
        let y = panel.y + panel.height + v_offset; 
        
        writeln!(
            svg, 
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" dominant-baseline="middle">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, label
        )?;
    } else {
        // --- PHYSICAL LEFT RENDERER ---
        let ticks = target_scale.ticks(8);
        
        // Compute the maximum horizontal width of rotated tick labels.
        let max_width = ticks.iter()
            .map(|t| {
                let w = crate::core::layout::estimate_text_width(&t.label, theme.tick_label_font_size);
                let h = theme.tick_label_font_size;
                w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
            })
            .fold(0.0, f64::max);

        // Total offset from the panel's left edge.
        let h_offset = tick_line_len + max_width + safety_buffer + label_padding + (theme.label_font_size / 2.0);
        let x = panel.x - h_offset; 
        let y = panel.y + panel.height / 2.0;
        
        // Vertical titles are rotated -90 degrees around their anchor point.
        writeln!(
            svg, 
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" transform="rotate(-90, {:.2}, {:.2})" dominant-baseline="middle">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, x, y, label
        )?;
    }
    Ok(())
}