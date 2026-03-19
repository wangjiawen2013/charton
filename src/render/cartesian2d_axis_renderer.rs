use crate::coordinate::{CoordinateTrait, Rect, cartesian::Cartesian2D};
use crate::error::ChartonError;
use crate::scale::ExplicitTick;
use crate::theme::Theme;
use html_escape::encode_safe;
use std::fmt::Write;

/// Orchestrates the visual rendering of both horizontal and vertical axes for a panel.
///
/// This function is "Panel-aware": it renders axes relative to the `Rect` provided
/// in the `PanelContext`. In a faceted chart, this is called for each individual panel.
#[allow(clippy::too_many_arguments)]
pub fn render_cartesian_axes(
    svg: &mut String,
    theme: &Theme,
    panel: &Rect,
    coord: &Cartesian2D,
    x_label: &str,
    x_explicit: Option<&[ExplicitTick]>,
    y_label: &str,
    y_explicit: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    // Determine which data label belongs to which physical position based on flip state.
    // If flipped, the Y-scale data is projected onto the visual Bottom axis.
    let (bottom_label, left_label) = if coord.is_flipped() {
        (y_label, x_label)
    } else {
        (x_label, y_label)
    };

    // Determine which explicit ticks belongs to which physical position based on flip state.
    let bottom_explicit = if coord.is_flipped() {
        y_explicit
    } else {
        x_explicit
    };
    let left_explicit = if coord.is_flipped() {
        x_explicit
    } else {
        y_explicit
    };

    // 1. Process the Physical Bottom Axis (X-axis in standard, Y-axis in flipped)
    draw_axis_line(svg, theme, panel, true)?;
    draw_ticks_and_labels(svg, theme, panel, coord, true, bottom_explicit)?;
    draw_axis_title(svg, theme, panel, coord, bottom_label, true)?;

    // 2. Process the Physical Left Axis (Y-axis in standard, X-axis in flipped)
    draw_axis_line(svg, theme, panel, false)?;
    draw_ticks_and_labels(svg, theme, panel, coord, false, left_explicit)?;
    draw_axis_title(svg, theme, panel, coord, left_label, false)?;

    Ok(())
}

/// Renders the straight line (spine) of the axis.
fn draw_axis_line(
    svg: &mut String,
    theme: &Theme,
    panel: &Rect,
    is_bottom: bool,
) -> Result<(), ChartonError> {
    let (x1, y1, x2, y2) = if is_bottom {
        // Horizontal line at the bottom edge of the panel
        (
            panel.x,
            panel.y + panel.height,
            panel.x + panel.width,
            panel.y + panel.height,
        )
    } else {
        // Vertical line at the left edge of the panel
        (panel.x, panel.y, panel.x, panel.y + panel.height)
    };

    writeln!(
        svg,
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}" stroke-linecap="square"/>"#,
        x1,
        y1,
        x2,
        y2,
        theme.label_color.to_css_string(),
        theme.axis_width
    )?;
    Ok(())
}

/// Renders the tick marks and their corresponding text labels.
///
/// This function handles the physical placement of ticks and labels.
/// Crucially, when `is_flipped` is true, it ensures that the data scale
/// and the visual rotation angle are correctly cross-referenced.
fn draw_ticks_and_labels(
    svg: &mut String,
    theme: &Theme,
    panel: &Rect,
    coord: &dyn CoordinateTrait,
    is_bottom: bool,
    explicit_ticks: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    let is_flipped = coord.is_flipped();

    // Select the logical scale currently occupying this physical axis.
    let target_scale = if is_flipped {
        if is_bottom {
            coord.get_y_scale()
        } else {
            coord.get_x_scale()
        }
    } else if is_bottom {
        coord.get_x_scale()
    } else {
        coord.get_y_scale()
    };

    let ticks = match explicit_ticks {
        // 1. User-provided explicit ticks: prioritize and process manual specifications.
        Some(explicit) => target_scale.create_explicit_ticks(explicit),

        // 2. Default: fallback to automatic tick suggestion based on available screen space.
        None => {
            let available_space = if is_bottom { panel.width } else { panel.height };
            let final_count = theme.suggest_tick_count(available_space);
            target_scale.suggest_ticks(final_count)
        }
    };

    let tick_len = 6.0;

    // FIX: The rotation angle must be resolved using the same logic as the scale.
    // If the chart is flipped, the labels on the bottom (horizontal) axis
    // are now representing the Y-dimension data and should use the Y-angle.
    let angle = if is_bottom {
        if is_flipped {
            theme.y_tick_label_angle
        } else {
            theme.x_tick_label_angle
        }
    } else if is_flipped {
        theme.x_tick_label_angle
    } else {
        theme.y_tick_label_angle
    };

    for tick in ticks {
        let norm_pos = target_scale.normalize(tick.value);

        let (px, py) = if is_bottom {
            (panel.x + norm_pos * panel.width, panel.y + panel.height)
        } else {
            (panel.x, panel.y + (1.0 - norm_pos) * panel.height)
        };

        let (x2, y2, dx, dy, anchor, baseline) = if is_bottom {
            // Anchor is 'end' for rotated text to ensure the string "hangs" off the tick properly.
            let x_anchor = if angle == 0.0 { "middle" } else { "end" };
            (
                px,
                py + tick_len,
                0.0,
                tick_len + theme.tick_label_padding,
                x_anchor,
                "hanging",
            )
        } else {
            // Left axis labels are typically right-aligned to the tick end.
            (
                px - tick_len,
                py,
                -(tick_len + theme.tick_label_padding + 1.0),
                0.0,
                "end",
                "central",
            )
        };

        writeln!(
            svg,
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}"/>"#,
            px,
            py,
            x2,
            y2,
            theme.label_color.to_css_string(),
            theme.tick_width
        )?;

        let final_x = px + dx;
        let final_y = py + dy;
        let transform = if angle != 0.0 {
            format!(
                r#" transform="rotate({:.1}, {:.2}, {:.2})""#,
                angle, final_x, final_y
            )
        } else {
            "".to_string()
        };

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}"{}>{}</text>"#,
            final_x,
            final_y,
            theme.tick_label_size,
            theme.tick_label_family,
            theme.tick_label_color.to_css_string(),
            anchor,
            baseline,
            transform,
            tick.label
        )?;
    }
    Ok(())
}

/// Renders the axis title, calculating offsets based on the bounding box of rotated tick labels.
fn draw_axis_title(
    svg: &mut String,
    theme: &Theme,
    panel: &Rect,
    coord: &dyn CoordinateTrait,
    label: &str,
    is_bottom: bool,
) -> Result<(), ChartonError> {
    if label.is_empty() {
        return Ok(());
    }

    let is_flipped = coord.is_flipped();

    let tick_line_len = 6.0;
    let title_gap = 5.0;

    // Resolve which angle and scale are mapped to this physical axis.
    // This logic MUST match draw_ticks_and_labels exactly.
    let (angle_rad, target_scale) = if is_flipped {
        if is_bottom {
            (theme.y_tick_label_angle.to_radians(), coord.get_y_scale())
        } else {
            (theme.x_tick_label_angle.to_radians(), coord.get_x_scale())
        }
    } else if is_bottom {
        (theme.x_tick_label_angle.to_radians(), coord.get_x_scale())
    } else {
        (theme.y_tick_label_angle.to_radians(), coord.get_y_scale())
    };

    let available_space = if is_bottom { panel.width } else { panel.height };
    let final_count = theme.suggest_tick_count(available_space);
    let ticks = target_scale.suggest_ticks(final_count);

    if is_bottom {
        let x = panel.x + panel.width / 2.0;

        // Calculate the maximum vertical extension (Descent) of the labels.
        // We use the absolute sine and cosine to find the total height of the rotated bounding box.
        let max_tick_height = ticks
            .iter()
            .map(|t| {
                let w = crate::core::utils::estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;

                // For a box (w, h) rotated by theta, the vertical projection is |w*sin(theta)| + |h*cos(theta)|.
                // This represents the total height occupied by the rotated text.
                w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
            })
            .fold(0.0, f64::max);

        // Compute total vertical offset from the panel edge.
        // Includes: tick lines + labels + theme padding + additional gap.
        let v_offset = tick_line_len + max_tick_height + theme.label_padding + title_gap;
        let y = panel.y + panel.height + v_offset;

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" dominant-baseline="hanging">{}</text>"#,
            x,
            y,
            theme.label_size,
            theme.label_family,
            theme.label_color.to_css_string(),
            encode_safe(label)
        )?;
    } else {
        let y = panel.y + panel.height / 2.0;

        // Calculate the maximum horizontal extension for the left axis.
        let max_tick_width = ticks
            .iter()
            .map(|t| {
                let w = crate::core::utils::estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;
                w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
            })
            .fold(0.0, f64::max);

        // For the vertical title, we rotate 90 degrees and align the center (middle baseline).
        // Therefore, we add half the label_size to ensure the title doesn't bleed into the labels.
        // + 3.0 for some extra padding, it's an empirical value.
        let h_offset = tick_line_len
            + max_tick_width
            + theme.label_padding
            + title_gap
            + (theme.label_size / 2.0)
            + 3.0;
        let x = panel.x - h_offset;

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" font-weight="bold" transform="rotate(-90, {:.2}, {:.2})" dominant-baseline="middle">{}</text>"#,
            x,
            y,
            theme.label_size,
            theme.label_family,
            theme.label_color.to_css_string(),
            x,
            y,
            encode_safe(label)
        )?;
    }

    Ok(())
}
