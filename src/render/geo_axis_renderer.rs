use crate::Precision;
use crate::coordinate::CoordinateTrait;
use crate::coordinate::geo::Geo;
use crate::core::layer::{LineConfig, RenderBackend, TextConfig};
use crate::error::ChartonError;
use crate::scale::ExplicitTick;
use crate::theme::Theme;

/// Renders geographic axes (longitude/latitude grid lines) for the Geo coordinate system.
///
/// Renders straight grid lines at the panel edges with tick labels.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_geo_axes(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &crate::coordinate::Rect,
    geo: &Geo,
    x_label: &str,
    x_explicit: Option<&[ExplicitTick]>,
    y_label: &str,
    y_explicit: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    let font_size = theme.tick_label_size;
    let text_color = theme.tick_label_color;
    let font_family = theme.tick_label_family.clone();
    let axis_width = theme.axis_width;
    let axis_color = SingleColor::new("#333333");
    let label_size = theme.label_size;
    let label_color = theme.label_color;
    let label_family = theme.label_family.clone();
    let tick_len = 6.0;
    let title_gap = 5.0;

    // --- X-axis (Longitude) ---
    let x_ticks = generate_ticks(geo, true, x_explicit);
    let y_bottom = panel.y + panel.height;

    // Axis line
    backend.draw_line(LineConfig {
        x1: panel.x as Precision,
        y1: y_bottom as Precision,
        x2: (panel.x + panel.width) as Precision,
        y2: y_bottom as Precision,
        color: axis_color,
        width: axis_width as Precision,
        opacity: 1.0,
        dash: vec![],
    });

    // Tick marks and labels
    for tick in &x_ticks {
        let tx = panel.x + tick.0 * panel.width;
        backend.draw_line(LineConfig {
            x1: tx as Precision,
            y1: y_bottom as Precision,
            x2: tx as Precision,
            y2: (y_bottom + tick_len) as Precision,
            color: axis_color,
            width: axis_width as Precision,
            opacity: 1.0,
            dash: vec![],
        });

        backend.draw_text(TextConfig {
            x: tx as Precision,
            y: (y_bottom + tick_len + theme.tick_label_padding) as Precision,
            text: tick.1.clone(),
            font_size: font_size as Precision,
            font_family: font_family.clone(),
            color: text_color,
            text_anchor: "middle".to_string(),
            dominant_baseline: "hanging".to_string(),
            font_weight: "normal".to_string(),
            opacity: 1.0,
            angle: 0.0,
        });
    }

    // X-axis label
    if !x_label.is_empty() {
        let max_tick_height = x_ticks
            .iter()
            .map(|t| {
                let w = crate::core::utils::estimate_text_width(&t.1, font_size);
                let h = font_size;
                w * 0.0 + h
            })
            .fold(0.0, f64::max);

        let label_x = panel.x + panel.width / 2.0;
        let label_y = y_bottom + tick_len + max_tick_height + theme.label_padding + title_gap;
        backend.draw_text(TextConfig {
            x: label_x as Precision,
            y: label_y as Precision,
            text: x_label.to_string(),
            font_size: label_size as Precision,
            font_family: label_family.clone(),
            color: label_color,
            text_anchor: "middle".to_string(),
            dominant_baseline: "hanging".to_string(),
            font_weight: "normal".to_string(),
            opacity: 1.0,
            angle: 0.0,
        });
    }

    // --- Y-axis (Latitude) ---
    let y_ticks = generate_ticks(geo, false, y_explicit);
    let x_left = panel.x;

    // Axis line
    backend.draw_line(LineConfig {
        x1: x_left as Precision,
        y1: panel.y as Precision,
        x2: x_left as Precision,
        y2: (panel.y + panel.height) as Precision,
        color: axis_color,
        width: axis_width as Precision,
        opacity: 1.0,
        dash: vec![],
    });

    // Tick marks and labels
    for tick in &y_ticks {
        let ty = panel.y + (1.0 - tick.0) * panel.height;
        backend.draw_line(LineConfig {
            x1: x_left as Precision,
            y1: ty as Precision,
            x2: (x_left - tick_len) as Precision,
            y2: ty as Precision,
            color: axis_color,
            width: axis_width as Precision,
            opacity: 1.0,
            dash: vec![],
        });

        backend.draw_text(TextConfig {
            x: (x_left - tick_len - theme.tick_label_padding) as Precision,
            y: ty as Precision,
            text: tick.1.clone(),
            font_size: font_size as Precision,
            font_family: font_family.clone(),
            color: text_color,
            text_anchor: "end".to_string(),
            dominant_baseline: "central".to_string(),
            font_weight: "normal".to_string(),
            opacity: 1.0,
            angle: 0.0,
        });
    }

    // Y-axis label
    if !y_label.is_empty() {
        let max_tick_width = y_ticks
            .iter()
            .map(|t| crate::core::utils::estimate_text_width(&t.1, font_size))
            .fold(0.0, f64::max);

        let label_x = x_left
            - tick_len
            - max_tick_width
            - theme.label_padding
            - title_gap
            - (label_size / 2.0)
            - 3.0;
        let label_y = panel.y + panel.height / 2.0;

        backend.draw_text(TextConfig {
            x: label_x as Precision,
            y: label_y as Precision,
            text: y_label.to_string(),
            font_size: label_size as Precision,
            font_family: label_family.clone(),
            color: label_color,
            text_anchor: "middle".to_string(),
            dominant_baseline: "middle".to_string(),
            font_weight: "normal".to_string(),
            opacity: 1.0,
            angle: -90.0,
        });
    }

    Ok(())
}

/// Renders geographic grid lines for the Geo coordinate system.
pub(crate) fn render_geo_grid(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &crate::coordinate::Rect,
    geo: &Geo,
    x_explicit: Option<&[ExplicitTick]>,
    y_explicit: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    let grid_color = theme.grid_color;
    let grid_width = theme.grid_width;

    let x_ticks = generate_ticks(geo, true, x_explicit);
    let y_ticks = generate_ticks(geo, false, y_explicit);

    // Vertical grid lines (longitude)
    for tick in &x_ticks {
        let tx = panel.x + tick.0 * panel.width;
        backend.draw_line(LineConfig {
            x1: tx as Precision,
            y1: panel.y as Precision,
            x2: tx as Precision,
            y2: (panel.y + panel.height) as Precision,
            color: grid_color,
            width: grid_width as Precision,
            opacity: 1.0,
            dash: vec![],
        });
    }

    // Horizontal grid lines (latitude)
    for tick in &y_ticks {
        let ty = panel.y + (1.0 - tick.0) * panel.height;
        backend.draw_line(LineConfig {
            x1: panel.x as Precision,
            y1: ty as Precision,
            x2: (panel.x + panel.width) as Precision,
            y2: ty as Precision,
            color: grid_color,
            width: grid_width as Precision,
            opacity: 1.0,
            dash: vec![],
        });
    }

    Ok(())
}

/// Generates normalized tick positions and labels for longitude or latitude axes.
fn generate_ticks(
    geo: &Geo,
    is_x_axis: bool,
    explicit: Option<&[ExplicitTick]>,
) -> Vec<(f64, String)> {
    if let Some(explicit_ticks) = explicit {
        return generate_explicit_ticks(geo, explicit_ticks, is_x_axis);
    }

    let scale: &dyn crate::scale::ScaleTrait = if is_x_axis {
        geo.get_x_scale()
    } else {
        geo.get_y_scale()
    };

    let ticks = scale.suggest_ticks(8);

    ticks
        .into_iter()
        .map(|t| {
            let norm = scale.normalize(t.value);
            (norm, t.label)
        })
        .collect()
}

fn generate_explicit_ticks(
    geo: &Geo,
    ticks: &[ExplicitTick],
    is_x_axis: bool,
) -> Vec<(f64, String)> {
    let scale: &dyn crate::scale::ScaleTrait = if is_x_axis {
        geo.get_x_scale()
    } else {
        geo.get_y_scale()
    };

    ticks
        .iter()
        .map(|tick| match tick {
            ExplicitTick::Continuous(v) => {
                let norm = scale.normalize(*v);
                (norm, format!("{:.1}", v))
            }
            ExplicitTick::Discrete(label) => {
                let norm = scale.normalize_string(label);
                (norm, label.clone())
            }
            ExplicitTick::Timestamp(ts) => {
                let norm = scale.normalize(*ts as f64);
                (norm, format!("{}", ts))
            }
            ExplicitTick::Temporal(dt) => {
                let norm = scale.normalize(dt.unix_timestamp() as f64);
                let label = format!(
                    "{:04}-{:02}-{:02}",
                    dt.year(),
                    u8::from(dt.month()),
                    dt.day()
                );
                (norm, label)
            }
        })
        .collect()
}

use crate::visual::color::SingleColor;
