use crate::coordinate::{Rect, CoordinateTrait, polar::Polar};
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/// Renders the polar coordinate system axes.
///
/// This implementation follows modern visualization standards (like ggplot2/Vega-Lite):
/// - Radial ticks are placed along the spine.
/// - Angular ticks are wrapped around the circumference.
/// - Axis titles are placed at non-intrusive locations to maximize readability.
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

    // Geometric constants
    let center_x = panel.x + panel.width / 2.0;
    let center_y = panel.y + panel.height / 2.0;
    let max_r = panel.width.min(panel.height) / 2.0;

    // --- 1. RADIAL AXIS (Concentric Circles & Radial Ticks) ---
    let y_ticks = y_scale.ticks(theme.suggest_tick_count(max_r));
    for tick in y_ticks {
        let y_n = y_scale.normalize(tick.value);
        let r_norm = coord.inner_radius + y_n * (1.0 - coord.inner_radius);
        let r_px = r_norm * max_r;

        if r_px <= 0.0 { continue; }

        // Grid circle
        writeln!(
            svg,
            r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="none" stroke="{}" stroke-width="{:.2}" stroke-dasharray="2,4" opacity="0.3"/>"#,
            center_x, center_y, r_px, theme.grid_color.to_css_string(), theme.grid_width
        )?;

        // Radial tick labels (placed on the start_angle ray)
        // We keep them horizontal for better readability (Best Practice)
        let tx = center_x + r_px * coord.start_angle.cos();
        let ty = center_y + r_px * coord.start_angle.sin();
        
        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="middle" dominant-baseline="middle" opacity="0.8">{}</text>"#,
            tx, ty, theme.tick_label_size - 2.0, theme.tick_label_family, 
            theme.tick_label_color.to_css_string(), tick.label
        )?;
    }

    // --- 2. ANGULAR AXIS (Spokes & Circular Ticks) ---
    let x_ticks = x_scale.ticks(theme.suggest_tick_count(2.0 * std::f64::consts::PI * max_r));
    for tick in x_ticks {
        let x_n = x_scale.normalize(tick.value);
        let theta = coord.start_angle + x_n * (coord.end_angle - coord.start_angle);
        
        let r_inner = coord.inner_radius * max_r;
        let x1 = center_x + r_inner * theta.cos();
        let y1 = center_y + r_inner * theta.sin();
        let x2 = center_x + max_r * theta.cos();
        let y2 = center_y + max_r * theta.sin();

        // Radial spoke (grid line)
        writeln!(
            svg,
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.2}" opacity="0.3"/>"#,
            x1, y1, x2, y2, theme.grid_color.to_css_string(), theme.grid_width
        )?;

        // Circumference labels
        let label_r = max_r + theme.tick_label_padding + 2.0;
        let lx = center_x + label_r * theta.cos();
        let ly = center_y + label_r * theta.sin();
        
        // Smart anchoring: text-anchor depends on which side of the circle we are on
        let cos_t = theta.cos();
        let anchor = if cos_t > 0.1 { "start" } else if cos_t < -0.1 { "end" } else { "middle" };

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="middle">{}</text>"#,
            lx, ly, theme.tick_label_size, theme.tick_label_family,
            theme.tick_label_color.to_css_string(), anchor, tick.label
        )?;
    }

    // --- 3. AXIS TITLES ---
    // In Polar coordinates (especially Pie/Donut), we typically skip rendering 
    // the generic axis labels (x_label, y_label) to maintain visual clarity.
    // The tick labels (categories around the circle) are usually sufficient.
    
    // Explicitly do nothing with x_label and y_label
    let _ = x_label;
    let _ = y_label;

    Ok(())
}
