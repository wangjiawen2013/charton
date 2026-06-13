use crate::Precision;
use crate::coordinate::{CoordinateTrait, Rect, cartesian::Cartesian2D};
use crate::core::layer::{PathConfig, PathTopology, RenderBackend, TextConfig};
use crate::error::ChartonError;
use crate::scale::ExplicitTick;
use crate::theme::Theme;

/// Orchestrates the visual rendering of both horizontal and vertical axes for a panel.
///
/// This function is "Panel-aware": it renders axes relative to the `Rect` provided
/// in the `PanelContext`. In a faceted chart, this is called for each individual panel.
#[allow(clippy::too_many_arguments)]
pub fn render_cartesian_axes(
    backend: &mut dyn RenderBackend, // A generic backend for rendering.
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
    draw_axis_line(backend, theme, panel, true)?;
    draw_ticks_and_labels(backend, theme, panel, coord, true, bottom_explicit)?;
    draw_axis_title(backend, theme, panel, coord, bottom_label, true)?;

    // 2. Process the Physical Left Axis (Y-axis in standard, X-axis in flipped)
    draw_axis_line(backend, theme, panel, false)?;
    draw_ticks_and_labels(backend, theme, panel, coord, false, left_explicit)?;
    draw_axis_title(backend, theme, panel, coord, left_label, false)?;

    Ok(())
}

/// Renders the straight line (spine) of the axis.
fn draw_axis_line(
    backend: &mut dyn RenderBackend,
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

    // Using PathConfig to draw the single line segment of the axis spine
    backend.draw_path(PathConfig {
        points: vec![
            (x1 as Precision, y1 as Precision),
            (x2 as Precision, y2 as Precision),
        ],
        fill: "none".into(),
        stroke: theme.label_color,
        stroke_width: theme.axis_width as Precision,
        opacity: 1.0,
        dash: vec![], // Solid line
        topology: PathTopology::Simple,
    });

    Ok(())
}

/// Renders the individual ticks and their associated labels.
fn draw_ticks_and_labels(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &Rect,
    coord: &dyn CoordinateTrait,
    is_bottom: bool,
    explicit_ticks: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    let is_flipped = coord.is_flipped();

    // 1. Select logical scale based on coordinate orientation
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

    // 2. Generate ticks based on available pixel space
    let ticks = match explicit_ticks {
        Some(explicit) => target_scale.create_explicit_ticks(explicit),
        None => {
            let available_space = if is_bottom { panel.width } else { panel.height };
            target_scale.suggest_ticks(theme.suggest_tick_count(available_space))
        }
    };

    let tick_len = 6.0;

    // 3. Resolve rotation angle for tick labels
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

        // --- DRAW TICK LINE ---
        let (x2, y2) = if is_bottom {
            (px, py + tick_len)
        } else {
            (px - tick_len, py)
        };

        backend.draw_path(PathConfig {
            points: vec![
                (px as Precision, py as Precision),
                (x2 as Precision, y2 as Precision),
            ],
            fill: "none".into(),
            stroke: theme.label_color,
            stroke_width: theme.tick_width as Precision,
            opacity: 1.0,
            dash: vec![],
            topology: PathTopology::Simple,
        });

        // --- DRAW TICK LABEL ---
        let (dx, dy, anchor, baseline) = if is_bottom {
            let x_anchor = if angle == 0.0 { "middle" } else { "end" };
            (
                0.0,
                tick_len + theme.tick_label_padding,
                x_anchor,
                "hanging",
            )
        } else {
            (
                -(tick_len + theme.tick_label_padding + 1.0),
                0.0,
                "end",
                "central",
            )
        };

        backend.draw_text(TextConfig {
            text: tick.label.clone(),
            x: (px + dx) as Precision,
            y: (py + dy) as Precision,
            font_size: theme.tick_label_size as Precision,
            font_family: theme.tick_label_family.clone(),
            color: theme.tick_label_color,
            text_anchor: anchor.to_string(),
            dominant_baseline: baseline.to_string(),
            font_weight: "normal".to_string(), // Ticks usually use normal weight
            opacity: 1.0,
            angle: angle as Precision,
        });
    }
    Ok(())
}

/// Renders the axis title, calculating offsets based on the bounding box of rotated tick labels.
fn draw_axis_title(
    backend: &mut dyn RenderBackend,
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

        // Calculate the maximum vertical extension (Descent) of the labels to avoid overlap.
        let max_tick_height = ticks
            .iter()
            .map(|t| {
                let w = crate::core::utils::estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;
                w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
            })
            .fold(0.0, f64::max);

        // Compute total vertical offset from the panel edge.
        let v_offset = tick_line_len + max_tick_height + theme.label_padding + title_gap;
        let y = panel.y + panel.height + v_offset;

        backend.draw_text(TextConfig {
            x: x as Precision,
            y: y as Precision,
            text: label.to_string(),
            font_size: theme.label_size as Precision,
            font_family: theme.label_family.clone(),
            color: theme.label_color,
            text_anchor: "middle".to_string(),
            dominant_baseline: "hanging".to_string(),
            font_weight: "bold".to_string(),
            opacity: 1.0,
            angle: 0.0,
        });
    } else {
        let y = panel.y + panel.height / 2.0;

        // Calculate the maximum horizontal extension for the left axis to prevent clipping.
        let max_tick_width = ticks
            .iter()
            .map(|t| {
                let w = crate::core::utils::estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;
                w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
            })
            .fold(0.0, f64::max);

        // Total horizontal offset for vertical axis title.
        let h_offset = tick_line_len
            + max_tick_width
            + theme.label_padding
            + title_gap
            + (theme.label_size / 2.0)
            + 3.0;
        let x = panel.x - h_offset;

        backend.draw_text(TextConfig {
            x: x as Precision,
            y: y as Precision,
            text: label.to_string(),
            font_size: theme.label_size as Precision,
            font_family: theme.label_family.clone(),
            color: theme.label_color,
            text_anchor: "middle".to_string(),
            dominant_baseline: "middle".to_string(),
            font_weight: "bold".to_string(),
            opacity: 1.0,
            angle: -90.0, // Rotate Counter-Clockwise(CCW) for vertical alignment
        });
    }

    Ok(())
}
