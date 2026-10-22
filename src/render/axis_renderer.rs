//! Drawing the value axes of a rectangular panel.
//!
//! Every rectangular coordinate system (Cartesian, Geo, and any future one)
//! draws its axes here. The renderer owns no layout decisions of its own: which
//! scale appears on a physical side, which labels to draw, where the labels
//! start and how deep the band is all come from [`AxisView`], the same object
//! the layout engine measured with. Drawing therefore cannot drift away from
//! the reserved space, and a coordinate system that joins the pipeline gets the
//! same tick density, rotation, explicit-tick and title handling as the others
//! for free.

use crate::Precision;
use crate::coordinate::{AxisVisibility, CoordinateTrait, Rect};
use crate::core::axis::{AxisSide, AxisView};
use crate::core::layer::{LineConfig, PathConfig, PathTopology, RenderBackend, TextConfig};
use crate::error::ChartonError;
use crate::scale::ExplicitTick;
use crate::theme::Theme;

/// Renders both value axes of one panel.
///
/// A faceted chart calls this once per panel, so everything is relative to the
/// `panel` rectangle. An axis is skipped entirely when `visibility` hides it.
pub fn render_axes(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &Rect,
    coord: &dyn CoordinateTrait,
    x_explicit: Option<&[ExplicitTick]>,
    y_explicit: Option<&[ExplicitTick]>,
    visibility: AxisVisibility,
) -> Result<(), ChartonError> {
    for (side, visible) in [
        (AxisSide::Bottom, visibility.show_x),
        (AxisSide::Left, visibility.show_y),
    ] {
        if !visible {
            continue;
        }

        let axis = AxisView::new(coord, side, panel, x_explicit, y_explicit, theme);
        draw_spine(backend, theme, panel, side)?;
        draw_ticks(backend, &axis)?;
        draw_title(backend, &axis)?;
    }

    Ok(())
}

/// Draws the straight line that closes the panel on one side.
fn draw_spine(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &Rect,
    side: AxisSide,
) -> Result<(), ChartonError> {
    let (x1, y1, x2, y2) = match side {
        AxisSide::Bottom => (
            panel.x,
            panel.y + panel.height,
            panel.x + panel.width,
            panel.y + panel.height,
        ),
        AxisSide::Left => (panel.x, panel.y, panel.x, panel.y + panel.height),
    };

    backend.draw_path(PathConfig {
        points: vec![
            (x1 as Precision, y1 as Precision),
            (x2 as Precision, y2 as Precision),
        ],
        fill: "none".into(),
        stroke: theme.axes_color,
        stroke_width: theme.axis_width as Precision,
        opacity: 1.0,
        dash: vec![],
        topology: PathTopology::Simple,
    });

    Ok(())
}

/// Draws the tick marks and their labels.
///
/// Both sides share one code path: [`AxisView::tick_point`] gives the position
/// on the spine and [`AxisView::band`] the distance out to the label anchor.
/// Only the text anchoring differs, because a bottom label hangs below its tick
/// while a left label ends at it.
fn draw_ticks(backend: &mut dyn RenderBackend, axis: &AxisView<'_>) -> Result<(), ChartonError> {
    let theme = axis.theme();
    let band = axis.band();
    let (out_x, out_y) = axis.side().outward();
    let tick_len = theme.tick_length;

    let (anchor, baseline) = match axis.side() {
        AxisSide::Bottom => (
            if axis.angle_deg() == 0.0 {
                "middle"
            } else {
                "end"
            },
            "hanging",
        ),
        AxisSide::Left => ("end", "central"),
    };

    for tick in axis.ticks() {
        let norm = axis.scale().normalize(tick.value);
        let (px, py) = axis.tick_point(norm);

        backend.draw_path(PathConfig {
            points: vec![
                (px as Precision, py as Precision),
                (
                    (px + out_x * tick_len) as Precision,
                    (py + out_y * tick_len) as Precision,
                ),
            ],
            fill: "none".into(),
            stroke: theme.tick_color,
            stroke_width: theme.tick_width as Precision,
            opacity: 1.0,
            dash: vec![],
            topology: PathTopology::Simple,
        });

        backend.draw_text(TextConfig {
            text: tick.label.clone(),
            x: (px + out_x * band.label_offset) as Precision,
            y: (py + out_y * band.label_offset) as Precision,
            font_size: theme.tick_label_size as Precision,
            font_family: theme.tick_label_family.clone(),
            color: theme.tick_label_color,
            text_anchor: anchor.to_string(),
            dominant_baseline: baseline.to_string(),
            font_weight: "normal".to_string(),
            font_style: "normal".to_string(),
            opacity: 1.0,
            angle: axis.angle_deg() as Precision,
        })?;
    }

    Ok(())
}

/// Draws the axis title in the strip the band reserved for it.
fn draw_title(backend: &mut dyn RenderBackend, axis: &AxisView<'_>) -> Result<(), ChartonError> {
    if axis.title().is_empty() {
        return Ok(());
    }

    let theme = axis.theme();
    let panel = axis.panel();
    let band = axis.band();

    // The bottom title hangs from the top of its strip, centred on the panel;
    // the left title is rotated and centred inside its strip.
    let (x, y, angle, anchor, baseline) = match axis.side() {
        AxisSide::Bottom => (
            panel.x + panel.width / 2.0,
            panel.y + panel.height + band.title_offset,
            0.0,
            "middle",
            "hanging",
        ),
        AxisSide::Left => (
            panel.x - (band.title_offset + band.title_height / 2.0),
            panel.y + panel.height / 2.0,
            -90.0,
            "middle",
            "middle",
        ),
    };

    backend.draw_text(TextConfig {
        x: x as Precision,
        y: y as Precision,
        text: axis.title().to_string(),
        font_size: theme.label_size as Precision,
        font_family: theme.label_family.clone(),
        color: theme.label_color,
        text_anchor: anchor.to_string(),
        dominant_baseline: baseline.to_string(),
        font_weight: "bold".to_string(),
        font_style: "normal".to_string(),
        opacity: 1.0,
        angle,
    })?;

    Ok(())
}

/// Renders the grid lines of a rectangular panel.
///
/// The grid is the labelled ticks extended across the panel, so it is built from
/// the same [`AxisView`]s as the axes. That keeps grid lines, tick marks and
/// labels on exactly the same values, including explicit tick lists.
pub fn render_grid(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    panel: &Rect,
    coord: &dyn CoordinateTrait,
    x_explicit: Option<&[ExplicitTick]>,
    y_explicit: Option<&[ExplicitTick]>,
) -> Result<(), ChartonError> {
    let bottom = AxisView::new(
        coord,
        AxisSide::Bottom,
        panel,
        x_explicit,
        y_explicit,
        theme,
    );
    for tick in bottom.ticks() {
        let (x, _) = bottom.tick_point(bottom.scale().normalize(tick.value));
        backend.draw_line(LineConfig {
            x1: x as Precision,
            y1: panel.y as Precision,
            x2: x as Precision,
            y2: (panel.y + panel.height) as Precision,
            color: theme.grid_color,
            width: theme.grid_width as Precision,
            opacity: 0.5,
            dash: vec![],
        });
    }

    let left = AxisView::new(coord, AxisSide::Left, panel, x_explicit, y_explicit, theme);
    for tick in left.ticks() {
        let (_, y) = left.tick_point(left.scale().normalize(tick.value));
        backend.draw_line(LineConfig {
            x1: panel.x as Precision,
            y1: y as Precision,
            x2: (panel.x + panel.width) as Precision,
            y2: y as Precision,
            color: theme.grid_color,
            width: theme.grid_width as Precision,
            opacity: 0.5,
            dash: vec![],
        });
    }

    // Close the frame even when no tick lands exactly on an edge.
    let right = panel.x + panel.width;
    let bottom_edge = panel.y + panel.height;
    for (x1, y1, x2, y2) in [
        (panel.x, panel.y, right, panel.y),
        (right, panel.y, right, bottom_edge),
        (right, bottom_edge, panel.x, bottom_edge),
        (panel.x, bottom_edge, panel.x, panel.y),
    ] {
        backend.draw_line(LineConfig {
            x1: x1 as Precision,
            y1: y1 as Precision,
            x2: x2 as Precision,
            y2: y2 as Precision,
            color: theme.grid_color,
            width: theme.grid_width as Precision,
            opacity: 0.5,
            dash: vec![],
        });
    }

    Ok(())
}
