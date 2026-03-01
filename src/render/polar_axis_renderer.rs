use crate::coordinate::{CoordinateTrait, Rect, polar::Polar};
use crate::error::ChartonError;
use crate::theme::Theme;
use std::fmt::Write;

/// Renders the polar coordinate system axes (Radial and Angular).
///
/// Refinements included:
/// 1. Radial Axis (Y): Only displays the maximum domain value to keep the center clean.
/// 2. Smart Positioning: Labels use quadrant-aware anchoring to "grow" away from lines.
/// 3. Padding: Added theme-based padding to prevent text from touching the outer ring.
pub fn render_polar_axes(
    svg: &mut String,
    theme: &Theme,
    panel: &Rect,
    coord: &Polar,
    x_label: &str,
    y_label: &str,
) -> Result<(), ChartonError> {
    let x_scale = coord.get_x_scale();
    let y_scale = coord.get_y_scale();

    // Geometric constants for polar-to-cartesian projection
    let center_x = panel.x + panel.width / 2.0;
    let center_y = panel.y + panel.height / 2.0;
    let max_r = panel.width.min(panel.height) / 2.0;

    // --- 1. RADIAL AXIS (Y-Axis) ---
    // We draw the outer boundary and a single label for the maximum value.

    // Draw the boundary circle (The 100% or Max limit line)
    writeln!(
        svg,
        r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="none" stroke="{}" stroke-width="{:.2}" opacity="0.5"/>"#,
        center_x,
        center_y,
        max_r,
        theme.grid_color.to_css_string(),
        theme.grid_width
    )?;

    // Fetch and format the Maximum Value using the Scale's internal formatter
    let y_domain = y_scale.domain();
    let max_val = y_domain.1;

    // In a Pie Chart, x_scale domain is typically empty and the sum at the edge is redundant.
    let is_pie = x_label.is_empty();

    let y_ticks = crate::scale::format_ticks(&[max_val]);
    let max_label = y_ticks.first().map(|t| t.label.as_str()).unwrap_or("");

    if !is_pie && !max_label.is_empty() {
        // Apply padding so the label floats just outside the max radius
        let label_r = max_r + theme.tick_label_padding + 2.0;
        let theta_start = coord.start_angle;

        let tx = center_x + label_r * theta_start.cos();
        let ty = center_y + label_r * theta_start.sin();

        // Quadrant-based alignment logic (Synchronized with X-axis logic below)
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
            "hanging" // Bottom: Text hangs below the point
        } else if sin_s < -0.5 {
            "auto" // Top: Text stands above the point
        } else {
            "middle" // Sides: Vertically centered
        };

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}" opacity="0.9">{}</text>"#,
            tx,
            ty,
            theme.tick_label_size - 1.0,
            theme.tick_label_family,
            theme.tick_label_color.to_css_string(),
            anchor,
            baseline,
            max_label
        )?;
    }

    // --- 2. ANGULAR AXIS (X-Axis) ---
    // Renders radial grid lines and circumferential category/value labels.
    let x_ticks = x_scale.ticks(theme.suggest_tick_count(2.0 * std::f64::consts::PI * max_r));

    for tick in x_ticks {
        let x_n = x_scale.normalize(tick.value);
        let theta = coord.start_angle + x_n * (coord.end_angle - coord.start_angle);

        // Calculate grid line endpoints
        let x1 = center_x + (coord.inner_radius * max_r) * theta.cos();
        let y1 = center_y + (coord.inner_radius * max_r) * theta.sin();
        let x2 = center_x + max_r * theta.cos();
        let y2 = center_y + max_r * theta.sin();

        // Radial Spokes (Grid lines separating sectors)
        writeln!(
            svg,
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.2}" opacity="0.5"/>"#,
            x1,
            y1,
            x2,
            y2,
            theme.grid_color.to_css_string(),
            theme.grid_width
        )?;

        // Circumference labels with "Smart Positioning"
        let label_r = max_r + theme.tick_label_padding + 2.0;
        let lx = center_x + label_r * theta.cos();
        let ly = center_y + label_r * theta.sin();

        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // Horizontal Anchoring: Decides if text grows left, right, or center
        let anchor = if cos_t > 0.1 {
            "start"
        } else if cos_t < -0.1 {
            "end"
        } else {
            "middle"
        };

        // Vertical Alignment: Prevents text from overlapping the circular boundary
        let baseline = if sin_t > 0.5 {
            "hanging" // Bottom labels
        } else if sin_t < -0.5 {
            "auto" // Top labels
        } else {
            "middle" // Side labels
        };

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}">{}</text>"#,
            lx,
            ly,
            theme.tick_label_size,
            theme.tick_label_family,
            theme.tick_label_color.to_css_string(),
            anchor,
            baseline,
            tick.label
        )?;
    }

    // Explicitly ignore labels to satisfy compiler if unused
    let _ = (x_label, y_label);

    Ok(())
}
