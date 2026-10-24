//! Drawing the angular axis of a polar panel.
//!
//! Polar charts have no axis band: values are placed by angle along the
//! circumference and by radius from the centre, and only the outer ring is
//! drawn (a radial axis would crowd the centre). The circle is described once by
//! [`PolarFrame`] and shared by the axis and the grid, and the tick count comes
//! from the same [`resolve_ticks`] policy as the rectangular axes.

use crate::Precision;
use crate::coordinate::{AxisVisibility, CoordinateTrait, Rect, polar::Polar};
use crate::core::layer::{CircleConfig, LineConfig, RenderBackend, TextConfig};
use crate::core::semantic::{AxisKind, Element, Group, Role};
use crate::core::ticks::{AxisDirection, resolve_ticks};
use crate::error::ChartonError;
use crate::scale::ExplicitTick;
use crate::theme::Theme;

/// Small gap between the ring and the tick labels.
const LABEL_GAP: f64 = 2.0;

/// The circle a polar panel inscribes: centre and radius.
///
/// The angular axis runs along its circumference, so both the axis and the grid
/// need the same geometry. Computing it in one place keeps the ring, the spokes
/// and the labels on the same circle.
struct PolarFrame {
    center_x: f64,
    center_y: f64,
    radius: f64,
}

impl PolarFrame {
    fn of(panel: &Rect) -> Self {
        Self {
            center_x: panel.x + panel.width / 2.0,
            center_y: panel.y + panel.height / 2.0,
            radius: panel.width.min(panel.height) / 2.0,
        }
    }

    /// Arc length available to the angular labels.
    fn circumference(&self) -> f64 {
        2.0 * std::f64::consts::PI * self.radius
    }
}

/// Renders the angular axis of a polar panel.
///
/// The polar model has no axis band: values are placed by angle on the
/// circumference and by radius from the centre, and only the outer ring is
/// drawn (a radial axis would clutter the centre). Tick labels are horizontal
/// and anchored by quadrant so they grow away from the ring.
pub fn render_polar_axes(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &Rect,
    coord: &Polar,
    x_explicit: Option<&[ExplicitTick]>,
    _y_explicit: Option<&[ExplicitTick]>,
    visibility: AxisVisibility,
) -> Result<(), ChartonError> {
    let x_scale = coord.get_x_scale();
    let frame = PolarFrame::of(panel);
    let (center_x, center_y, max_r) = (frame.center_x, frame.center_y, frame.radius);

    // Draw the outermost boundary circle. The ring is the angular axis' spine,
    // so it shares the axis scope with the angular labels below.
    if visibility.show_x || visibility.show_y {
        backend.begin_group(&Group::new(Role::Axis(AxisKind::Theta)));
        backend.begin_group(&Group::new(Role::AxisLine));
        backend.draw_circle(CircleConfig {
            x: center_x as Precision,
            y: center_y as Precision,
            radius: max_r as Precision,
            fill: "none".into(),
            stroke: theme.grid_color,
            stroke_width: theme.grid_width as Precision,
            opacity: 0.5,
        });
        backend.end_group();
    }

    // An empty x label means a pie/donut, whose slices are labelled by the
    // legend. Angular ticks would only clutter the ring.
    let is_pie_or_donut = coord.get_x_label().is_empty();

    if visibility.show_x && !is_pie_or_donut {
        backend.begin_group(&Group::new(Role::TickLabel));
        for tick in angular_ticks(coord, &frame, x_explicit, theme) {
            let x_n = x_scale.normalize(tick.value);
            let theta = coord.start_angle + x_n * (coord.end_angle - coord.start_angle);

            // Calculate label coordinates with padding
            let label_r = max_r + theme.tick_label_padding + LABEL_GAP;
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

            backend.begin_element(Element::new(&tick.label));
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
                font_style: "normal".to_string(),
                opacity: 1.0,
                angle: 0.0,
            })?;
            backend.end_element();
        }
        backend.end_group();
    }

    if visibility.show_x || visibility.show_y {
        backend.end_group();
    }

    Ok(())
}

/// The angular ticks, shared by the axis and its grid.
///
/// The room one label gets is arc length, not a straight panel edge, so the
/// resolver is asked with the circumference. Labels are drawn horizontally,
/// hence `AxisDirection::Horizontal` and no rotation.
fn angular_ticks(
    coord: &Polar,
    frame: &PolarFrame,
    explicit: Option<&[ExplicitTick]>,
    theme: &Theme,
) -> Vec<crate::scale::Tick> {
    resolve_ticks(
        coord.get_x_scale(),
        explicit,
        frame.circumference(),
        AxisDirection::Horizontal,
        0.0,
        theme,
    )
}

/// Renders the underlying grid: the outer ring plus one spoke per angular tick.
/// Polar has no cartesian grid; these spokes are its equivalent.
pub fn render_polar_grid(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &Rect,
    coord: &Polar,
    x_explicit: Option<&[ExplicitTick]>,
    _y_explicit: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    let frame = PolarFrame::of(panel);
    let (center_x, center_y, max_r) = (frame.center_x, frame.center_y, frame.radius);

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
    for tick in angular_ticks(coord, &frame, x_explicit, theme) {
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
