use crate::chart::common::SharedRenderingContext;
use crate::error::ChartonError;
use crate::render::utils::estimate_text_width;
use crate::theme::Theme;
use std::fmt::Write;

// Render both x and y axes for the chart
pub(crate) fn render_axes(
    svg: &mut String,
    theme: &Theme,
    context: &SharedRenderingContext,
) -> Result<(), ChartonError> {
    render_x_axis(svg, theme, context)?;
    render_y_axis(svg, theme, context)?;
    Ok(())
}

// Renders the x-axis with ticks and labels
fn render_x_axis(
    svg: &mut String,
    theme: &Theme,
    context: &SharedRenderingContext,
) -> Result<(), ChartonError> {
    // Get styling properties from theme
    let tick_label_font_size = theme.tick_label_font_size;
    let tick_label_font_family = theme.tick_label_font_family.clone();
    let tick_label_color = theme.tick_label_color.clone();
    let tick_label_angle = theme.x_tick_label_angle;

    // Use custom axis stroke width or fall back to theme default
    let axis_stroke_width = theme.axis_stroke_width;
    let tick_stroke_width = theme.tick_stroke_width;

    // Use a default axis color and tick length
    let axis_color = "black".to_string();
    let tick_length = 5.0;
    let tick_label_spacing = 3.0;

    // Determine axis position and orientation based on swapped_axes from context
    let (axis_line_x1, axis_line_y1, axis_line_x2, axis_line_y2, tick_direction) =
        if context.swapped_axes {
            // When swapped, x-axis is vertical (typically on the left)
            (
                context.draw_x0,
                context.draw_y0,
                context.draw_x0,
                context.draw_y0 + context.plot_height,
                -1.0,
            ) // Ticks to the left
        } else {
            // Horizontal axis on the bottom
            (
                context.draw_x0,
                context.draw_y0 + context.plot_height,
                context.draw_x0 + context.plot_width,
                context.draw_y0 + context.plot_height,
                1.0,
            ) // Ticks downward
        };

    // Draw axis line
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
        axis_line_x1, axis_line_y1, axis_line_x2, axis_line_y2, axis_color, axis_stroke_width
    )?;

    if context.swapped_axes {
        // For swapped axes, x-axis is vertical
        // Find the maximum length of x-axis tick labels to adjust positioning
        let max_tick_label_length = context
            .coord_system
            .x_axis
            .explicit_ticks
            .ticks
            .iter()
            .map(|tick| estimate_text_width(&tick.label, tick_label_font_size as f64))
            .fold(0.0, f64::max);

        // Draw ticks and labels for vertical x-axis
        for tick in &context.coord_system.x_axis.explicit_ticks.ticks {
            // Vertical axis with ticks to the left
            let y_pos = (context.x_mapper)(tick.position);

            // Draw tick line
            writeln!(
                svg,
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
                axis_line_x1 + tick_length * tick_direction,
                y_pos,
                axis_line_x1,
                y_pos,
                axis_color,
                tick_stroke_width
            )?;

            // Draw tick label
            writeln!(
                svg,
                r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="end" dominant-baseline="middle">{}</text>"#,
                axis_line_x1 + (tick_length * tick_direction) - tick_label_spacing,
                y_pos,
                tick_label_font_size,
                tick_label_font_family,
                tick_label_color,
                tick.label
            )?;
        }

        // Draw axis label if needed
        let label_font_size = theme.label_font_size;
        let label_font_family = theme.label_font_family.clone();
        let label_color = theme.label_color.clone();
        let label_padding = theme.x_label_padding;

        let label = &context.coord_system.x_axis.label;

        // Vertical title on the left side - account for max tick label width
        let label_x_pos =
            axis_line_x1 + (tick_length * tick_direction) - max_tick_label_length - label_padding;
        let label_y_pos = context.draw_y0 + context.plot_height / 2.0;

        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="middle" dominant-baseline="text-after-edge" transform="rotate(-90, {}, {})">{}</text>"#,
            label_x_pos,
            label_y_pos,
            label_font_size,
            label_font_family,
            label_color,
            label_x_pos,
            label_y_pos,
            label
        )?;
    } else {
        // Normal horizontal x-axis
        // Find the maximum width of x-axis tick labels to adjust positioning (for rotated labels)
        let max_tick_label_length = context
            .coord_system
            .x_axis
            .explicit_ticks
            .ticks
            .iter()
            .map(|tick| estimate_text_width(&tick.label, tick_label_font_size as f64))
            .fold(0.0, f64::max);
        let max_tick_label_height = if tick_label_angle.abs() > 1e-10 {
            // For rotated labels, estimate height based on text length and rotation angle
            (max_tick_label_length + tick_label_font_size as f64)
                * tick_label_angle.to_radians().sin().abs()
        } else {
            // For non-rotated labels, use font size
            tick_label_font_size as f64
        };

        // Draw ticks and labels
        for tick in &context.coord_system.x_axis.explicit_ticks.ticks {
            // Horizontal axis with ticks downward
            let x_pos = (context.x_mapper)(tick.position);

            // Draw tick line
            writeln!(
                svg,
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
                x_pos,
                axis_line_y1,
                x_pos,
                axis_line_y1 + tick_length * tick_direction,
                axis_color,
                tick_stroke_width
            )?;

            // Draw tick label
            let y_pos =
                axis_line_y1 + tick_length * tick_direction + tick_label_font_size as f64 + 0.0;

            if tick_label_angle.abs() < 1e-10 {
                // No rotation
                writeln!(
                    svg,
                    r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="middle">{}</text>"#,
                    x_pos,
                    y_pos,
                    tick_label_font_size,
                    tick_label_font_family,
                    tick_label_color,
                    tick.label
                )?;
            } else {
                // With rotation, 3.0 is a hack(offset) to align the text with the tick
                writeln!(
                    svg,
                    r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="end" transform="rotate({}, {}, {})">{}</text>"#,
                    x_pos + 3.0,
                    y_pos,
                    tick_label_font_size,
                    tick_label_font_family,
                    tick_label_color,
                    tick_label_angle,
                    x_pos,
                    y_pos,
                    tick.label
                )?;
            }
        }

        // Draw axis label
        let label_font_size = theme.label_font_size;
        let label_font_family = theme.label_font_family.clone();
        let label_color = theme.label_color.clone();
        let label_padding = theme.x_label_padding;

        let label = &context.coord_system.x_axis.label;

        // Horizontal title below the axis - account for max tick label height
        let label_x_pos = context.draw_x0 + context.plot_width / 2.0;
        let label_y_pos =
            axis_line_y1 + tick_length * tick_direction + max_tick_label_height + label_padding
                - 10.0;

        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="middle" dominant-baseline="text-before-edge">{}</text>"#,
            label_x_pos, label_y_pos, label_font_size, label_font_family, label_color, label
        )?;
    }

    Ok(())
}

// Renders the y-axis with ticks and labels
fn render_y_axis(
    svg: &mut String,
    theme: &Theme,
    context: &SharedRenderingContext,
) -> Result<(), ChartonError> {
    // Get styling properties from theme
    let tick_label_font_size = theme.tick_label_font_size;
    let tick_label_font_family = theme.tick_label_font_family.clone();
    let tick_label_color = theme.tick_label_color.clone();
    let tick_label_angle = theme.y_tick_label_angle;

    let axis_stroke_width = theme.axis_stroke_width;
    let tick_stroke_width = theme.tick_stroke_width;

    // Use a default axis color and tick length
    let axis_color = "black".to_string();
    let tick_length = 5.0;
    let tick_label_spacing = 3.0;

    // Determine axis position and orientation based on swapped_axes from context
    let (axis_line_x1, axis_line_y1, axis_line_x2, axis_line_y2, tick_direction) =
        if context.swapped_axes {
            // When swapped, y-axis is horizontal (typically on the bottom)
            (
                context.draw_x0,
                context.draw_y0 + context.plot_height,
                context.draw_x0 + context.plot_width,
                context.draw_y0 + context.plot_height,
                1.0,
            ) // Ticks upward
        } else {
            // Vertical axis on the left
            (
                context.draw_x0,
                context.draw_y0,
                context.draw_x0,
                context.draw_y0 + context.plot_height,
                -1.0,
            ) // Ticks to the left
        };

    // Draw axis line
    writeln!(
        svg,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
        axis_line_x1, axis_line_y1, axis_line_x2, axis_line_y2, axis_color, axis_stroke_width
    )?;

    if context.swapped_axes {
        // For swapped axes, y-axis is horizontal
        // Find the maximum height of y-axis tick labels to adjust positioning
        let max_tick_label_length = context
            .coord_system
            .y_axis
            .explicit_ticks
            .ticks
            .iter()
            .map(|tick| estimate_text_width(&tick.label, tick_label_font_size as f64))
            .fold(0.0, f64::max);
        let max_tick_label_height = if tick_label_angle.abs() > 1e-10 {
            // For rotated labels, estimate height based on text length and rotation angle
            (max_tick_label_length + tick_label_font_size as f64)
                * tick_label_angle.to_radians().sin().abs()
        } else {
            // For non-rotated labels, use font size
            tick_label_font_size as f64
        };

        // Draw ticks and labels for horizontal y-axis
        for tick in &context.coord_system.y_axis.explicit_ticks.ticks {
            // Horizontal axis with ticks upward
            let x_pos = (context.y_mapper)(tick.position);

            // Draw tick line
            writeln!(
                svg,
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
                x_pos,
                axis_line_y1,
                x_pos,
                axis_line_y1 + tick_length * tick_direction,
                axis_color,
                tick_stroke_width
            )?;

            // Draw tick label
            if tick_label_angle.abs() < 1e-10 {
                // No rotation
                let y_pos = axis_line_y1 + tick_length * tick_direction + tick_label_spacing;
                writeln!(
                    svg,
                    r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="middle" dominant-baseline="text-before-edge">{}</text>"#,
                    x_pos,
                    y_pos,
                    tick_label_font_size,
                    tick_label_font_family,
                    tick_label_color,
                    tick.label
                )?;
            } else {
                // With rotation
                let y_pos =
                    axis_line_y1 + tick_length * tick_direction + tick_label_font_size as f64;
                writeln!(
                    svg,
                    r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="end" transform="rotate({}, {}, {})">{}</text>"#,
                    x_pos + 3.0,
                    y_pos,
                    tick_label_font_size,
                    tick_label_font_family,
                    tick_label_color,
                    tick_label_angle,
                    x_pos,
                    y_pos,
                    tick.label
                )?;
            }
        }

        // Draw axis label
        let label_font_size = theme.label_font_size;
        let label_font_family = theme.label_font_family.clone();
        let label_color = theme.label_color.clone();
        let label_padding = theme.y_label_padding;

        let label = &context.coord_system.y_axis.label;

        // Horizontal title below the axis - account for max tick label height
        let label_x_pos = context.draw_x0 + context.plot_width / 2.0;
        let label_y_pos =
            axis_line_y1 + tick_length * tick_direction + max_tick_label_height + label_padding
                - 10.0;

        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="middle" dominant-baseline="text-before-edge">{}</text>"#,
            label_x_pos, label_y_pos, label_font_size, label_font_family, label_color, label
        )?;
    } else {
        // Normal vertical y-axis
        // Find the maximum length of y-axis tick labels to adjust positioning
        let max_tick_label_length = context
            .coord_system
            .y_axis
            .explicit_ticks
            .ticks
            .iter()
            .map(|tick| estimate_text_width(&tick.label, tick_label_font_size as f64))
            .fold(0.0, f64::max);

        // Draw ticks and labels
        for tick in &context.coord_system.y_axis.explicit_ticks.ticks {
            // Vertical axis with ticks to the left
            let y_pos = (context.y_mapper)(tick.position);

            // Draw tick line
            writeln!(
                svg,
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
                axis_line_x1 + tick_length * tick_direction,
                y_pos,
                axis_line_x1,
                y_pos,
                axis_color,
                tick_stroke_width
            )?;

            // Draw tick label
            if tick_label_angle.abs() < 1e-10 {
                // No rotation
                writeln!(
                    svg,
                    r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="end" dominant-baseline="middle">{}</text>"#,
                    axis_line_x1 + (tick_length * tick_direction) - tick_label_spacing,
                    y_pos,
                    tick_label_font_size,
                    tick_label_font_family,
                    tick_label_color,
                    tick.label
                )?;
            } else {
                // With rotation
                let label_x_pos =
                    axis_line_x1 + (tick_length * tick_direction) - tick_label_spacing;
                writeln!(
                    svg,
                    r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="end" transform="rotate({}, {}, {})">{}</text>"#,
                    label_x_pos,
                    y_pos,
                    tick_label_font_size,
                    tick_label_font_family,
                    tick_label_color,
                    tick_label_angle,
                    label_x_pos,
                    y_pos,
                    tick.label
                )?;
            }
        }

        // Draw axis label
        let label_font_size = theme.label_font_size;
        let label_font_family = theme.label_font_family.clone();
        let label_color = theme.label_color.clone();
        let label_padding = theme.y_label_padding;

        let label = &context.coord_system.y_axis.label;

        // Vertical title on the left side - account for max tick label width
        let label_x_pos =
            axis_line_x1 + (tick_length * tick_direction) - max_tick_label_length - label_padding;
        let label_y_pos = context.draw_y0 + context.plot_height / 2.0;

        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="middle" transform="rotate(-90, {}, {})">{}</text>"#,
            label_x_pos,
            label_y_pos,
            label_font_size,
            label_font_family,
            label_color,
            label_x_pos,
            label_y_pos,
            label
        )?;
    }

    Ok(())
}
