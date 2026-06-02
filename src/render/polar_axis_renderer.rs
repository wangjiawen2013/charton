use crate::Precision;
use crate::coordinate::{CoordinateTrait, Rect, polar::Polar};
use crate::core::layer::{PathTopology, CircleConfig, PathConfig, RenderBackend, TextConfig};
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
    y_label: &str,
    _y_explicit: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    let x_scale = coord.get_x_scale();
    let y_scale = coord.get_y_scale();

    // Calculate geometric center and maximum radius of the polar chart
    let center_x = panel.x + panel.width / 2.0;
    let center_y = panel.y + panel.height / 2.0;
    let max_r = panel.width.min(panel.height) / 2.0;

    // --- SECTION 1: RADIAL AXIS (Y-Axis / Circles) ---

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

    // Determine the maximum value for the radial label
    let y_domain = y_scale.domain();
    let max_val = y_domain.1;

    // Skip redundant labels if it's a Pie chart (indicated by empty x_label)
    let is_pie = x_label.is_empty();
    let y_ticks = crate::scale::format_ticks(&[max_val]);
    let max_label = y_ticks.first().map(|t| t.label.as_str()).unwrap_or("");

    if !is_pie && !max_label.is_empty() {
        // Place the label slightly outside the max radius using theme padding
        let label_r = max_r + theme.tick_label_padding + 2.0;
        let theta_start = coord.start_angle;

        let tx = center_x + label_r * theta_start.cos();
        let ty = center_y + label_r * theta_start.sin();

        // Quadrant-aware logic: determines alignment based on the label's angle
        let cos_s = theta_start.cos();
        let sin_s = theta_start.sin();

        let anchor = if cos_s > 0.1 {
            "start"
        } else if cos_s < -0.1 {
            "end"
        } else {
            "middle"
        };
        let baseline = if sin_s > 0.5 {
            "hanging"
        } else if sin_s < -0.5 {
            "auto"
        } else {
            "middle"
        };

        backend.draw_text(TextConfig {
            x: tx as Precision,
            y: ty as Precision,
            text: max_label.to_string(),
            font_size: (theme.tick_label_size - 1.0) as Precision,
            font_family: theme.tick_label_family.clone(),
            color: theme.tick_label_color,
            text_anchor: anchor.to_string(),
            dominant_baseline: baseline.to_string(),
            font_weight: "normal".to_string(),
            opacity: 0.9,
            angle: 0.0, // Keep text horizontal for readability
        });
    }

    // --- SECTION 2: ANGULAR AXIS (X-Axis / Spokes) ---

    // Generate ticks for the circumference based on the total perimeter
    let x_ticks =
        x_scale.suggest_ticks(theme.suggest_tick_count(2.0 * std::f64::consts::PI * max_r));

    for tick in x_ticks {
        let x_n = x_scale.normalize(tick.value);
        let theta = coord.start_angle + x_n * (coord.end_angle - coord.start_angle);

        // Project radial line coordinates from inner_radius to max_radius
        let x1 = center_x + (coord.inner_radius * max_r) * theta.cos();
        let y1 = center_y + (coord.inner_radius * max_r) * theta.sin();
        let x2 = center_x + max_r * theta.cos();
        let y2 = center_y + max_r * theta.sin();

        // Draw radial grid lines (spokes) separating different categories/sectors
        backend.draw_path(PathConfig {
            points: vec![
                (x1 as Precision, y1 as Precision),
                (x2 as Precision, y2 as Precision),
            ],
            stroke: theme.grid_color,
            stroke_width: theme.grid_width as Precision,
            opacity: 0.5,
            dash: vec![], // Solid line
            topology: PathTopology::Simple,
        });

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

    // Silence unused parameter warnings for future label implementations
    let _ = (x_label, y_label);

    Ok(())
}
