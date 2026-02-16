use crate::coordinate::{Rect, CoordinateTrait, polar::Polar};
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/// Renders the polar coordinate system axes.
///
/// This implementation ensures that:
/// 1. Radial ticks (Y-axis) are rendered UNDER the data marks to avoid cluttering sectors.
/// 2. Angular labels (X-axis) use "Smart Anchoring" to grow away from the boundary circle.
/// 3. Vertical alignment (baseline) is adjusted so text never overlaps the outer ring.
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

    // --- 1. RADIAL AXIS (Y-Axis) ---
    // Note: Rendered first so that data marks (rendered later) can mask internal labels.
    let y_ticks = y_scale.ticks(theme.suggest_tick_count(max_r));
    
    // Boundary circle at 1.0 normalization.
    writeln!(
        svg,
        r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="none" stroke="{}" stroke-width="{:.2}" opacity="0.5"/>"#,
        center_x, center_y, max_r, 
        theme.grid_color.to_css_string(), theme.grid_width
    )?;

    for tick in y_ticks {
        let y_n = y_scale.normalize(tick.value);
        let r_px = (coord.inner_radius + y_n * (1.0 - coord.inner_radius)) * max_r;

        if r_px <= 0.0 { continue; }

        let tx = center_x + r_px * coord.start_angle.cos();
        let ty = center_y + r_px * coord.start_angle.sin();
        
        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="middle" dominant-baseline="middle" opacity="0.8">{}</text>"#,
            tx, ty, theme.tick_label_size - 2.0, theme.tick_label_family, 
            theme.tick_label_color.to_css_string(), tick.label
        )?;
    }

    // --- 2. ANGULAR AXIS (X-Axis) ---
    let x_ticks = x_scale.ticks(theme.suggest_tick_count(2.0 * std::f64::consts::PI * max_r));
    for tick in x_ticks {
        let x_n = x_scale.normalize(tick.value);
        let theta = coord.start_angle + x_n * (coord.end_angle - coord.start_angle);
        
        let x1 = center_x + (coord.inner_radius * max_r) * theta.cos();
        let y1 = center_y + (coord.inner_radius * max_r) * theta.sin();
        let x2 = center_x + max_r * theta.cos();
        let y2 = center_y + max_r * theta.sin();

        // Radial spokes (Grid lines)
        writeln!(
            svg,
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.2}" opacity="0.3"/>"#,
            x1, y1, x2, y2, theme.grid_color.to_css_string(), theme.grid_width
        )?;

        // Circumference labels with "Smart Positioning"
        let label_r = max_r + theme.tick_label_padding;
        let lx = center_x + label_r * theta.cos();
        let ly = center_y + label_r * theta.sin();
        
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // 💡 HORIZONTAL ANCHORING
        // start: text is to the right of point | end: text is to the left | middle: centered
        let anchor = if cos_t > 0.1 { "start" } else if cos_t < -0.1 { "end" } else { "middle" };

        // 💡 VERTICAL ALIGNMENT (Dominant Baseline)
        // hanging: text hangs below point (for bottom labels)
        // auto/alphabetic: text stands above point (for top labels)
        // middle: text centered on point (for side labels)
        let baseline = if sin_t > 0.5 { 
            "hanging"  // Bottom labels: push text FURTHER DOWN
        } else if sin_t < -0.5 { 
            "auto"     // Top labels: push text FURTHER UP
        } else { 
            "middle"   // Side labels: center vertically
        };

        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}">{}</text>"#,
            lx, ly, theme.tick_label_size, theme.tick_label_family,
            theme.tick_label_color.to_css_string(), anchor, baseline, tick.label
        )?;
    }

    // Explicitly ignore labels to satisfy compiler
    let _ = (x_label, y_label);

    Ok(())
}
