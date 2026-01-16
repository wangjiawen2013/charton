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
            (0.0, theme.tick_label_font_size + theme.tick_label_padding - 3.0) 
        } else { 
            //(-(theme.tick_label_padding + 10.0), theme.tick_label_font_size * 0.35) 
            (-(theme.tick_label_padding + 10.0), 0.0) 
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

/// Draws the axis title (X or Y label) with dynamic collision avoidance.
///
/// This function calculates the optimal placement for axis titles by measuring 
/// the space occupied by tick labels. 
///
/// For the X-axis: It ensures the title is placed below the tick labels by 
/// accounting for the label font size and user-defined padding.
///
/// For the Y-axis: It performs a "Look-Ahead" on tick labels using `estimate_text_width` 
/// to push the title leftwards, preventing overlap with varying data lengths.
///
/// # Arguments
/// * `svg` - The mutable string buffer to append SVG elements to.
/// * `theme` - Visual configuration including font families, sizes, and padding.
/// * `ctx` - Shared context providing access to the panel dimensions and coordinate scales.
/// * `label` - The text string to display (e.g., "Weight (1000 lbs)").
/// * `is_visual_x` - Direction flag: true for horizontal (bottom), false for vertical (left).
fn draw_axis_title(
    svg: &mut String,
    theme: &Theme,
    ctx: &SharedRenderingContext,
    label: &str,
    is_visual_x: bool,
) -> Result<(), ChartonError> {
    // Exit early if there is no label to render.
    if label.is_empty() { return Ok(()); }
    
    let panel = &ctx.panel;
    let coord = ctx.coord;

    // Standard metric for tick line length
    let tick_line_len = 6.0;
    let safety_buffer = 5.0;

    if is_visual_x {
        // --- HORIZONTAL (X) AXIS TITLE ---
        
        // 1. Center the text horizontally relative to the plotting panel.
        let x = panel.x + panel.width / 2.0;
        
        // 2. Dynamic Vertical Offset:
        // We need to clear the Tick Line and the Tick Labels.
        // For horizontal text, the 'height' is roughly the font size.
        // We also add half the title's own font size to measure padding from the edge.
        let title_half_thickness = theme.label_font_size / 2.0;
        let tick_label_height = theme.tick_label_font_size;

        // Total vertical offset from the bottom axis line:
        // offset = tick_line + tick_label_height + buffer + user_padding + title_half_thickness
        let v_offset = tick_line_len + tick_label_height + safety_buffer + theme.x_label_padding + title_half_thickness;
        
        let y = panel.y + panel.height + v_offset; 
        
        writeln!(
            svg, 
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, label
        )?;
    } else {
        // --- VERTICAL (Y) AXIS TITLE ---
        
        // 1. Resolve the active scale for the visual left axis.
        let target_scale = if coord.is_flipped() { 
            coord.get_x_scale() 
        } else { 
            coord.get_y_scale() 
        };
        
        // 2. Dynamic Width Measurement:
        // Identify the widest tick label to prevent title collision using 
        // the character-weighted estimation logic.
        let ticks = target_scale.ticks(8);
        let max_tick_width = ticks.iter()
            .map(|t| crate::core::layout::estimate_text_width(&t.label, theme.tick_label_font_size))
            .fold(0.0, f64::max);

        // 3. Coordinate Calculation (Edge-based Offset):
        // Calculate the offset to the title's center, ensuring padding 
        // is measured from the title's right edge.
        let title_half_thickness = theme.label_font_size / 2.0;

        // Total Horizontal Offset from the left axis line:
        let h_offset = tick_line_len + max_tick_width + safety_buffer + theme.y_label_padding + title_half_thickness;
        
        let x = panel.x - h_offset; 
        let y = panel.y + panel.height / 2.0;
        
        // 4. SVG Rendering with -90 degree rotation.
        writeln!(
            svg, 
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" transform="rotate(-90, {:.2}, {:.2})">{}</text>"#,
            x, y, theme.label_font_size, theme.label_font_family, theme.label_color, x, y, label
        )?;
    }
    Ok(())
}