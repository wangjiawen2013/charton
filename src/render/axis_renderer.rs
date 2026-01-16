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

/// Renders tick marks and their text labels.
///
/// This function dynamically selects the scale based on the flip state to ensure 
/// the correct data mapping is shown on the correct visual axis.
fn draw_ticks_and_labels(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    is_visual_x: bool,
) -> Result<(), ChartonError> {
    let coord = ctx.coord;
    let panel = &ctx.panel;
    
    // Resolve which scale belongs to this visual orientation.
    // If flipped: Bottom Axis -> Y Scale, Left Axis -> X Scale.
    let target_scale = if ctx.coord.is_flipped() {
        if is_visual_x { coord.get_y_scale() } else { coord.get_x_scale() }
    } else {
        if is_visual_x { coord.get_x_scale() } else { coord.get_y_scale() }
    };
    
    // Generate ticks based on the target scale's domain.
    let ticks = target_scale.ticks(8); 

    for tick in ticks {
        let norm_pos = target_scale.normalize(tick.value);

        // transform() handles the internal mapping of normalized values to pixels.
        // It accounts for flipped coordinates and Y-axis inversion.
        let (px, py) = if is_visual_x {
            coord.transform(norm_pos, 0.0, panel)
        } else {
            coord.transform(0.0, norm_pos, panel)
        };

        // --- TICK MARK ---
        let tick_len = 6.0;
        let (x2, y2) = if is_visual_x { (px, py + tick_len) } else { (px - tick_len, py) };
        
        writeln!(
            svg, 
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}"/>"#,
            px, py, x2, y2, theme.label_color, theme.tick_stroke_width
        )?;

        // --- TICK LABEL ---
        let anchor = if is_visual_x { "middle" } else { "end" };
        
        // Use theme paddings to position labels away from the spines.
        let (dx, dy) = if is_visual_x { 
            (0.0, theme.tick_label_font_size + theme.tick_label_padding) 
        } else { 
            (-(theme.tick_label_padding + 2.0), theme.tick_label_font_size * 0.35) 
        };

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}">{}</text>"#,
            px + dx, py + dy, theme.tick_label_font_size, theme.tick_label_font_family,
            theme.tick_label_color, anchor, if is_visual_x { "hanging" } else { "middle" }, tick.label
        )?;
    }
    Ok(())
}

/// Draws the axis title (X or Y label).
fn draw_axis_title(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    label: &str,
    is_visual_x: bool,
) -> Result<(), ChartonError> {
    if label.is_empty() { return Ok(()); }
    let panel = &ctx.panel;

    if is_visual_x {
        // Positioned centrally below the plotting panel.
        let x = panel.x + panel.width / 2.0;
        let y = panel.y + panel.height + theme.x_label_padding + 25.0; 
        
        writeln!(
            svg, 
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, label
        )?;
    } else {
        // Positioned to the left of the plotting panel and rotated 90 degrees.
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