use crate::Precision;
use crate::coordinate::{CoordinateTrait, Rect, polar::Polar};
use crate::core::layer::{CircleConfig, LineConfig, RenderBackend, TextConfig};
use crate::error::ChartonError;
use crate::scale::ExplicitTick;
use crate::theme::Theme;

/// Renders the polar coordinate system axes (Radial and Angular).
///
/// Refinements included:
/// 1. Radial Axis (Y): Only displays the maximum domain value to keep the center clean.
/// 2. Smart Positioning: Labels use quadrant-aware anchoring to "grow" away from lines.
/// 3. Padding: Added theme-based padding to prevent text from touching the outer ring.
#[allow(clippy::too_many_arguments)]
pub fn render_polar_axes(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &Rect,
    coord: &Polar,
    x_label: &str,
    _x_explicit: Option<&[ExplicitTick]>,
    _y_label: &str,
    _y_explicit: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    let x_scale = coord.get_x_scale();

    // Calculate geometric center and maximum radius of the polar chart
    let center_x = panel.x + panel.width / 2.0;
    let center_y = panel.y + panel.height / 2.0;
    let max_r = panel.width.min(panel.height) / 2.0;

    // Draw the outermost boundary circle
    backend.draw_circle(CircleConfig {
        x: center_x as Precision,
        y: center_y as Precision,
        radius: max_r as Precision,
        fill: "none".into(),
        stroke: theme.grid_color,
        stroke_width: theme.grid_width as Precision,
        opacity: 0.5,
    });

    // Check if the chart is a Pie or Donut chart (implied by an empty x_label).
    // For Nightingale Rose charts, we generate ticks along the circumference based on the total perimeter.
    // For Pie and Donut charts, we skip tick generation to avoid cluttering the visual display.
    let is_pie_or_donut = x_label.is_empty();
    let x_ticks = if !is_pie_or_donut {
        x_scale.suggest_ticks(theme.suggest_tick_count(2.0 * std::f64::consts::PI * max_r))
    } else {
        vec![]
    };

    for tick in x_ticks {
        let x_n = x_scale.normalize(tick.value);
        let theta = coord.start_angle + x_n * (coord.end_angle - coord.start_angle);

        // Calculate label coordinates with padding
        let label_r = max_r + theme.tick_label_padding + 2.0;
        let lx = center_x + label_r * theta.cos();
        let ly = center_y + label_r * theta.sin();

        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // Resolve text anchoring: ensures text "grows" away from the circle center
        // to prevent overlapping with grid lines.
        let anchor = if cos_t > 0.1 {
            "start"
        } else if cos_t < -0.1 {
            "end"
        } else {
            "middle"
        };
        let baseline = if sin_t > 0.5 {
            "hanging"
        } else if sin_t < -0.5 {
            "auto"
        } else {
            "middle"
        };

        backend.draw_text(TextConfig {
            x: lx as Precision,
            y: ly as Precision,
            text: tick.label.clone(),
            font_size: theme.tick_label_size as Precision,
            font_family: theme.tick_label_family.clone(),
            color: theme.tick_label_color,
            text_anchor: anchor.to_string(),
            dominant_baseline: baseline.to_string(),
            font_weight: "normal".to_string(),
            opacity: 1.0,
            angle: 0.0,
        });
    }

    Ok(())
}

/// Renders the underlying grid system for polar coordinates, including
/// concentric circular boundaries and angular spokes.
pub fn render_polar_grid(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &Rect,
    coord: &Polar,
    x_explicit: Option<&[ExplicitTick]>,
    y_explicit: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    let center_x = panel.x + panel.width / 2.0;
    let center_y = panel.y + panel.height / 2.0;
    let max_r = panel.width.min(panel.height) / 2.0;
    let _ = x_explicit;
    let _ = y_explicit;

    // 1. Draw the concentric background circles (text labels removed)
    backend.draw_circle(CircleConfig {
        x: center_x as Precision,
        y: center_y as Precision,
        radius: max_r as Precision,
        fill: "none".into(),
        stroke: theme.grid_color,
        stroke_width: theme.grid_width as Precision,
        opacity: 0.7,
    });

    // 2. Draw the angular grid lines / spokes (text labels removed)
    let x_scale = coord.get_x_scale();
    let x_ticks =
        x_scale.suggest_ticks(theme.suggest_tick_count(2.0 * std::f64::consts::PI * max_r));

    for tick in x_ticks {
        let x_n = x_scale.normalize(tick.value);
        let theta = coord.start_angle + x_n * (coord.end_angle - coord.start_angle);

        let x1 = center_x + (coord.inner_radius * max_r) * theta.cos();
        let y1 = center_y + (coord.inner_radius * max_r) * theta.sin();
        let x2 = center_x + max_r * theta.cos();
        let y2 = center_y + max_r * theta.sin();

        backend.draw_line(LineConfig {
            x1: x1 as Precision,
            y1: y1 as Precision,
            x2: x2 as Precision,
            y2: y2 as Precision,
            color: theme.grid_color,
            width: theme.grid_width as Precision,
            opacity: 0.5, // Grid lines can be slightly fainter than axis lines
            dash: vec![],
        });
    }

    Ok(())
}
