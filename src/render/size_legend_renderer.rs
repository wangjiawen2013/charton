use crate::chart::common::{Chart, SharedRenderingContext};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::render::constants::render_constants::SPACING;
use crate::theme::Theme;
use std::fmt::Write;

// Renders a size legend for continuous size scales
pub(crate) fn render_size_legend<T: Mark>(
    svg: &mut String,
    chart: &Chart<T>,
    theme: &Theme,
    context: &SharedRenderingContext,
) -> Result<(), ChartonError> {
    // Only render size legend if there's a size encoding with continuous scale
    let size_enc = match &chart.encoding.size {
        Some(enc) => enc,
        None => return Ok(()),
    };

    let size_series = chart.data.column(&size_enc.field)?;
    let scale_type = crate::data::determine_scale_for_dtype(size_series.dtype());

    // Only render size legend for continuous scales
    if scale_type == crate::coord::Scale::Discrete {
        return Ok(());
    }

    // Calculate plot area dimensions for proper alignment
    let plot_h = context.plot_height;
    let draw_x0 = context.draw_x0;
    let draw_y0 = context.draw_y0;
    let plot_w = context.plot_width;

    // Size legend dimensions and positioning
    let legend_width = 20.0;
    let legend_x = draw_x0 + plot_w + SPACING; // Add small padding

    // Calculate the height that matches the y-axis
    let legend_height = plot_h;

    // Use 45% of the axis height for the size legend + title
    let legend_section_height = legend_height * 0.45;
    // Leave some space for the title
    let title_height = 20.0;
    let actual_legend_height = legend_section_height - title_height;

    // Position size legend starting at 50% of the y-axis height
    let legend_y = draw_y0 + (plot_h * 0.50);

    // Render size legend title
    let title = &size_enc.field;
    let font_size = theme.legend_font_size.unwrap_or(theme.label_font_size);
    let font_family = theme
        .legend_font_family
        .as_deref()
        .unwrap_or(&theme.label_font_family);

    // Position title
    writeln!(
        svg,
        r#"<text x="{}" y="{}" font-size="{}" font-family="{}" text-anchor="left" font-weight="bold">{}</text>"#,
        legend_x,
        legend_y + title_height - 10.0,
        font_size,
        font_family,
        title
    )?;

    // Position the size legend just below the title
    let legend_start_y = legend_y + title_height + 3.0;

    // Get min and max values for size mapping
    let min_val = size_series.min::<f64>()?.unwrap();
    let max_val = size_series.max::<f64>()?.unwrap();

    // Define reference sizes for the legend (five equally spaced points)
    let reference_sizes = vec![
        (0.00, min_val + (max_val - min_val) * 0.00, 2.0), // Minimum size
        (0.25, min_val + (max_val - min_val) * 0.25, 4.0), // 25% size
        (0.50, min_val + (max_val - min_val) * 0.50, 6.0), // 50% size
        (0.75, min_val + (max_val - min_val) * 0.75, 8.0), // 75% size
        (1.00, min_val + (max_val - min_val) * 1.00, 10.0), // Maximum size
    ];

    let tick_length = 2.0;
    let tick_font_size = theme.tick_label_font_size;
    let tick_font_family = &theme.tick_label_font_family;
    let tick_color = &theme.tick_label_color;

    // Draw reference point (using default color and shape, but no fill)
    let point_stroke = chart
        .mark
        .as_ref()
        .and_then(|point_mark| point_mark.stroke().cloned())
        .unwrap_or_else(|| crate::visual::color::SingleColor::new("black"));

    let point_shape = chart
        .mark
        .as_ref()
        .map(|point_mark| point_mark.shape())
        .unwrap_or(crate::visual::shape::PointShape::Circle);

    // Draw reference points with their values
    for (i, (_ratio, value, size)) in reference_sizes.iter().enumerate() {
        let spacing_factor = (actual_legend_height / (reference_sizes.len() - 1) as f64) - 2.0;
        let y_pos = legend_start_y + i as f64 * spacing_factor;

        // Draw tick lines pointing inward from both sides
        writeln!(
            svg,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="white" stroke-width="1"/>"#,
            legend_x,
            y_pos,
            legend_x + tick_length,
            y_pos
        )?;

        writeln!(
            svg,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="white" stroke-width="1"/>"#,
            legend_x + legend_width - tick_length,
            y_pos,
            legend_x + legend_width,
            y_pos
        )?;

        // Render point with no fill by using None for fill_color
        crate::render::point_renderer::render_point(
            svg,
            legend_x + legend_width / 2.0,
            y_pos,
            &None, // No fill color to make it hollow
            &point_shape,
            *size,
            1.0,
            &Some(point_stroke.clone()),
            1.0,
        )?;

        // Draw tick label
        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" dominant-baseline="middle" text-anchor="start">{:.2}</text>"#,
            legend_x + legend_width + tick_length + 2.0,
            y_pos,
            tick_font_size,
            tick_font_family,
            tick_color,
            value
        )?;
    }

    Ok(())
}
