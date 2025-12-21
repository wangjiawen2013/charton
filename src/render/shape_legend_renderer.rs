use crate::chart::common::{Chart, SharedRenderingContext};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::render::constants::render_constants::*;
use crate::render::utils::estimate_text_width;
use crate::theme::Theme;
use std::fmt::Write;

// Renders a legend for shape encoding
pub(crate) fn render_shape_legend<T: Mark>(
    svg: &mut String,
    chart: &Chart<T>,
    theme: &Theme,
    context: &SharedRenderingContext,
) -> Result<(), ChartonError> {
    // Only render legend if there's a shape encoding
    let shape_enc = match &chart.encoding.shape {
        Some(enc) => enc,
        None => return Ok(()),
    };

    // Get shape data
    let shape_series = chart.data.column(&shape_enc.field)?;

    // Get unique shape values in order of appearance
    let unique_shapes_series = shape_series.unique_stable()?;
    let unique_shapes = unique_shapes_series
        .str()?
        .into_no_null_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // Use values from context and theme instead of hardcoded values
    let plot_h = context.plot_height;
    let draw_x0 = context.draw_x0;
    let draw_y0 = context.draw_y0;
    let plot_w = context.plot_width;

    // Legend positioning - use right margin area based on effective_right_margin
    let legend_x = draw_x0 + plot_w + SPACING; // Add small padding
    let legend_y = draw_y0;
    let available_height = plot_h;

    // Legend title
    let title = &shape_enc.field;
    let font_size = theme.legend_font_size.unwrap_or(theme.label_font_size);
    let font_family = theme
        .legend_font_family
        .as_deref()
        .unwrap_or(&theme.label_font_family);

    writeln!(
        svg,
        r#"<text x="{}" y="{}" font-size="{}" font-family="{}" text-anchor="start" font-weight="bold">{}</text>"#,
        legend_x,
        legend_y + (font_size as f64),
        font_size,
        font_family,
        title
    )?;

    // Multi-column legend setup
    let available_vertical_space = available_height - (font_size as f64) - 5.0; // Subtract space for title
    let max_items_per_column = (available_vertical_space / ITEM_HEIGHT).floor() as usize;

    // Ensure we have at least one item per column and respect the maximum items per column
    let max_items_per_column = max_items_per_column.clamp(1, MAX_ITEMS_PER_COLUMN);

    // Calculate column width based on available space and max label width
    let max_label_width = unique_shapes
        .iter()
        .map(|label| estimate_text_width(label, theme.tick_label_font_size as f64))
        .fold(0.0, f64::max);

    let column_width = COLOR_BOX_SIZE + COLOR_BOX_SPACING + max_label_width + LABEL_PADDING;

    // Available shapes for mapping data values to visual shapes
    let available_shapes = crate::visual::shape::PointShape::LEGEND_SHAPES;
    const SHAPE_SIZE: f64 = 6.0;

    // Render legend items in columns
    for (index, shape_category) in unique_shapes.iter().enumerate() {
        let column = index / max_items_per_column;
        let row = index % max_items_per_column;

        let item_x = legend_x + (column as f64) * (column_width + COLUMN_SPACING);
        let item_y = legend_y + (font_size as f64) + 10.0 + (row as f64) * ITEM_HEIGHT;

        // Systematically map category to a shape
        let shape_index = index % available_shapes.len();
        let shape = available_shapes[shape_index].clone();

        // Draw shape representation with only stroke, no fill
        crate::render::point_renderer::render_point(
            svg,
            item_x + COLOR_BOX_SIZE / 2.0,
            item_y + ITEM_HEIGHT / 2.0,
            &None, // No fill color
            &shape,
            SHAPE_SIZE,
            1.0,
            &Some(crate::visual::color::SingleColor::new("#000000")), // Black stroke
            1.0,
        )?;

        // Draw label - vertically centered with the shape box
        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" dominant-baseline="middle">{}</text>"#,
            item_x + COLOR_BOX_SIZE + COLOR_BOX_SPACING,
            item_y + ITEM_HEIGHT / 2.0,
            theme.tick_label_font_size,
            theme.tick_label_font_family,
            shape_category
        )?;
    }

    Ok(())
}
