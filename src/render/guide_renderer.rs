use crate::coordinate::{CoordinateTrait, Rect};
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/* * GUIDES DEFINITION:
 * In the Grammar of Graphics, "Guides" are visual elements that help the viewer 
 * map physical distances (pixels) back to data values. 
 * - Axes are guides for position scales.
 * - Legends are guides for aesthetic scales (color, shape, etc.).
 */

/// Renders the coordinate axis system (both X and Y).
/// 
/// This is the primary entry point for drawing the grid system based on 
/// the current coordinate mapping and the provided physical panel area.
pub fn render_axes(
    svg: &mut String,
    theme: &Theme,
    coord: &dyn CoordinateTrait,
    panel: &Rect,
) -> Result<(), ChartonError> {
    let (x_scale, y_scale) = coord.get_scales();

    // 1. Render X-Axis: Maps x-scale ticks to the panel.
    // By default, it assumes the baseline is at the start of the Y range (bottom).
    render_axis(
        svg, 
        theme, 
        coord, 
        panel, 
        x_scale.ticks(10), 
        true
    )?;

    // 2. Render Y-Axis: Maps y-scale ticks to the panel.
    // By default, it assumes the baseline is at the start of the X range (left).
    render_axis(
        svg, 
        theme, 
        coord, 
        panel, 
        y_scale.ticks(10), 
        false
    )?;

    Ok(())
}

/// Helper function to render a single axis (either horizontal or vertical).
fn render_axis(
    svg: &mut String,
    theme: &Theme,
    coord: &dyn CoordinateTrait,
    panel: &Rect,
    ticks: Vec<crate::scale::Tick>,
    is_x_axis: bool,
) -> Result<(), ChartonError> {
    let (x_scale, y_scale) = coord.get_scales();

    for tick in ticks {
        // Step 1: Normalization and Transformation
        // The CoordinateTrait handles the heavy lifting of pixel placement.
        let (px, py) = if is_x_axis {
            let norm = x_scale.normalize(tick.value);
            coord.transform(norm, 0.0, panel)
        } else {
            let norm = y_scale.normalize(tick.value);
            coord.transform(0.0, norm, panel)
        };

        // --- 3. Render Tick Mark (the small line) ---
        let tick_len = 5.0;
        let (x2, y2) = if is_x_axis {
            (px, py + tick_len)
        } else {
            (px - tick_len, py)
        };

        writeln!(
            svg,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
            px, py, x2, y2, theme.label_color, theme.tick_stroke_width
        )?;

        // --- 4. Render Tick Label (the text) ---
        let anchor = if is_x_axis { "middle" } else { "end" };
        
        let (dx, dy) = if is_x_axis {
            (0.0, theme.tick_label_font_size as f64 + 2.0)
        } else {
            (-8.0, 4.0) 
        };

        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="{}" dominant-baseline="{}">{}</text>"#,
            px + dx, 
            py + dy, 
            theme.tick_label_font_size, 
            theme.tick_label_font_family,
            theme.label_color, 
            anchor,
            if is_x_axis { "auto" } else { "middle" },
            tick.label
        )?;
    }

    Ok(())
}