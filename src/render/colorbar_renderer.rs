use crate::chart::common::{Chart, SharedRenderingContext};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::render::constants::render_constants::SPACING;
use crate::theme::Theme;
use std::fmt::Write;

// Renders a colorbar for continuous color scales
pub(crate) fn render_colorbar<T: Mark>(
    svg: &mut String,
    chart: &Chart<T>,
    theme: &Theme,
    context: &SharedRenderingContext,
) -> Result<(), ChartonError> {
    // Only render colorbar if there's a color encoding with continuous scale
    let color_enc = match &chart.encoding.color {
        Some(enc) => enc,
        None => return Ok(()),
    };

    let color_series = chart.data.column(&color_enc.field)?;
    let scale_type = crate::data::determine_scale_for_dtype(color_series.dtype());

    // Only render colorbar for continuous scales
    if scale_type == crate::coord::Scale::Discrete { return Ok(()) }

    // Calculate plot area dimensions for proper alignment
    let plot_h = context.plot_height;
    let draw_x0 = context.draw_x0;
    let draw_y0 = context.draw_y0;
    let plot_w = context.plot_width;

    // Colorbar dimensions and positioning
    let colorbar_width = 20.0;
    let colorbar_x = draw_x0 + plot_w + SPACING;

    // Calculate the height that matches the y-axis regardless of swapped axes
    let colorbar_height = plot_h;

    // Use 40% of the axis height for the colorbar + title
    let colorbar_section_height = colorbar_height * 0.4;
    // Leave some space for the colorbar title
    let title_height = 20.0;
    let actual_colorbar_height = colorbar_section_height - title_height;

    // Position colorbar to match the y-axis height (accounting for swapped axes)
    let colorbar_y = draw_y0;

    // Render colorbar title
    let title = &color_enc.field;
    let font_size = theme.legend_font_size.unwrap_or(theme.label_font_size);
    let font_family = theme
        .legend_font_family
        .as_deref()
        .unwrap_or(&theme.label_font_family);

    // Position title at the top of the colorbar section (adjusted for new title height)
    writeln!(
        svg,
        r#"<text x="{}" y="{}" font-size="{}" font-family="{}" text-anchor="left" font-weight="bold">{}</text>"#,
        colorbar_x,
        colorbar_y + title_height - 8.0,
        font_size,
        font_family,
        title
    )?;

    // Position the colorbar just below the title
    let colorbar_start_y = colorbar_y + title_height;

    // Render colorbar gradient
    let gradient_id = format!("colorbar-gradient-{}", title.replace(" ", "-"));

    writeln!(svg, r#"<defs>"#)?;
    // We want high values at the top and low values at the bottom
    writeln!(
        svg,
        r#"  <linearGradient id="{}" x1="0" y1="0" x2="0" y2="1">"#,
        gradient_id
    )?;

    // Generate gradient stops based on the colormap - but in reverse order
    let steps = 10;
    for i in 0..=steps {
        let ratio = i as f64 / steps as f64;
        // Fix: Reverse the color mapping so high values appear at the top
        let color = chart.mark_cmap.get_color(1.0 - ratio);
        writeln!(
            svg,
            r#"    <stop offset="{}%" stop-color="{}"/>"#,
            ratio * 100.0,
            color
        )?;
    }

    writeln!(svg, r#"  </linearGradient>"#)?;
    writeln!(svg, r#"</defs>"#)?;

    // Draw the colorbar rectangle with gradient fill
    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="url('#{}')"/>"#,
        colorbar_x, colorbar_start_y, colorbar_width, actual_colorbar_height, gradient_id
    )?;

    // Draw border around colorbar
    writeln!(
        svg,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="black" stroke-width="1"/>"#,
        colorbar_x, colorbar_start_y, colorbar_width, actual_colorbar_height
    )?;

    // Get min and max values for ticks
    let min_val = color_series.min::<f64>()?.unwrap();
    let max_val = color_series.max::<f64>()?.unwrap();

    // Render ticks and labels at 15%, 40%, 65%, 90% positions of the colorbar
    let tick_positions = vec![
        (0.15, min_val + (max_val - min_val) * 0.85), // 15% position
        (0.40, min_val + (max_val - min_val) * 0.60), // 40% position
        (0.65, min_val + (max_val - min_val) * 0.35), // 65% position
        (0.90, min_val + (max_val - min_val) * 0.10), // 90% position
    ];

    let tick_length = 4.0; // Extend ticks inward
    let tick_font_size = theme.tick_label_font_size;
    let tick_font_family = &theme.tick_label_font_family;
    let tick_color = &theme.tick_label_color;

    for (ratio, value) in tick_positions {
        let y_pos = colorbar_start_y + actual_colorbar_height * ratio;

        // Draw tick lines pointing inward from both sides
        writeln!(
            svg,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="white" stroke-width="1"/>"#,
            colorbar_x,
            y_pos,
            colorbar_x + tick_length,
            y_pos
        )?;

        writeln!(
            svg,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="white" stroke-width="1"/>"#,
            colorbar_x + colorbar_width - tick_length,
            y_pos,
            colorbar_x + colorbar_width,
            y_pos
        )?;

        // Draw tick label
        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" dominant-baseline="middle" text-anchor="start">{:.2}</text>"#,
            colorbar_x + colorbar_width + tick_length + 2.0,
            y_pos,
            tick_font_size,
            tick_font_family,
            tick_color,
            value
        )?;
    }

    Ok(())
}
