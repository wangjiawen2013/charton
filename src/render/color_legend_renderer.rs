use crate::chart::common::{Chart, SharedRenderingContext};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::render::constants::render_constants::*;
use crate::render::utils::estimate_text_width;
use crate::theme::Theme;
use std::fmt::Write;

// Renders a legend for discrete color scales
pub(crate) fn render_color_legend<T: Mark>(
    svg: &mut String,
    chart: &Chart<T>,
    theme: &Theme,
    context: &SharedRenderingContext,
) -> Result<(), ChartonError> {
    // Only render legend if there's a color encoding with discrete scale
    let color_enc = match &chart.encoding.color {
        Some(enc) => enc,
        None => return Ok(()),
    };

    let color_series = chart.data.column(&color_enc.field)?;
    let scale_type = crate::data::determine_scale_for_dtype(color_series.dtype());

    // Only render legend for discrete scales
    if !matches!(scale_type, crate::coord::Scale::Discrete) {
        return Ok(());
    }

    // Get unique color values to determine if we should show legend
    let unique_series = color_series.unique_stable()?;
    let unique = unique_series.str()?.into_no_null_iter().collect::<Vec<_>>();

    // Check if legend should be shown based on user preference or default logic
    let should_show_legend = match context.legend {
        Some(show) => show,       // User explicitly set show/hide
        None => unique.len() > 1, // Default: show only if more than one group
    };

    if !should_show_legend {
        return Ok(());
    }

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
    let title = &color_enc.field;
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
    let max_items_per_column = max_items_per_column.max(1).min(MAX_ITEMS_PER_COLUMN);

    // Calculate column width based on available space and max label width
    let max_label_width = unique
        .iter()
        .map(|label| estimate_text_width(label, theme.tick_label_font_size as f64))
        .fold(0.0, f64::max);

    let column_width = COLOR_BOX_SIZE + COLOR_BOX_SPACING + max_label_width + LABEL_PADDING;

    // Get opacity from the mark
    let opacity = if let Some(mark) = &chart.mark {
        mark.opacity()
    } else {
        1.0
    };

    // Draw mark using the same shape as the main chart
    let mark_type = chart.mark.as_ref().unwrap().mark_type();

    // Render legend items in columns
    for (index, value) in unique.iter().enumerate() {
        let column = index / max_items_per_column;
        let row = index % max_items_per_column;

        let item_x = legend_x + (column as f64) * (column_width + COLUMN_SPACING);
        let item_y = legend_y + (font_size as f64) + 10.0 + (row as f64) * ITEM_HEIGHT;

        // Get color for this value using direct indexing
        let unique_index = unique.iter().position(|x| *x == *value).unwrap_or(0);
        let color = chart.mark_palette.get_color(unique_index);

        // Fixed position for the color box - vertically centered in the item slot
        let box_center_y = item_y + ITEM_HEIGHT / 2.0;

        match mark_type {
            // For point mark, draw a small circle by default
            "point" => {
                writeln!(
                    svg,
                    r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="black" stroke-width="0.0" fill-opacity="{}"/>"#,
                    item_x + COLOR_BOX_SIZE / 2.0,
                    box_center_y,
                    COLOR_BOX_SIZE / 2.0,
                    color,
                    opacity
                )?;
            }
            "rect" => {
                writeln!(
                    svg,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="black" stroke-width="0.0" fill-opacity="{}"/>"#,
                    item_x,
                    box_center_y - COLOR_BOX_SIZE / 2.0,
                    COLOR_BOX_SIZE,
                    COLOR_BOX_SIZE,
                    color,
                    opacity
                )?;
            }
            "bar" => {
                writeln!(
                    svg,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="black" stroke-width="0.0" fill-opacity="{}"/>"#,
                    item_x,
                    box_center_y - COLOR_BOX_SIZE / 2.0,
                    COLOR_BOX_SIZE,
                    COLOR_BOX_SIZE,
                    color,
                    opacity
                )?;
            }
            "line" => {
                // For line mark, draw a small line segment
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" stroke-linecap="round" fill-opacity="{}"/>"#,
                    item_x,
                    box_center_y,
                    item_x + COLOR_BOX_SIZE,
                    box_center_y,
                    color,
                    opacity
                )?;
            }
            "area" => {
                // For area mark, draw a small rectangle
                writeln!(
                    svg,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="black" stroke-width="0.0" fill-opacity="{}"/>"#,
                    item_x,
                    box_center_y - COLOR_BOX_SIZE / 2.0,
                    COLOR_BOX_SIZE,
                    COLOR_BOX_SIZE,
                    color,
                    opacity
                )?;
            }
            "rule" => {
                // For rule mark, draw a small line segment
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" stroke-linecap="round" fill-opacity="{}"/>"#,
                    item_x,
                    box_center_y,
                    item_x + COLOR_BOX_SIZE,
                    box_center_y,
                    color,
                    opacity
                )?;
            }
            "tick" => {
                // For tick mark, draw a small vertical line
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" stroke-linecap="round" fill-opacity="{}"/>"#,
                    item_x + COLOR_BOX_SIZE / 2.0,
                    box_center_y - COLOR_BOX_SIZE / 2.0,
                    item_x + COLOR_BOX_SIZE / 2.0,
                    box_center_y + COLOR_BOX_SIZE / 2.0,
                    color,
                    opacity
                )?;
            }
            _ => {
                // Default to rectangle for unknown mark types
                writeln!(
                    svg,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="black" stroke-width="0.0" fill-opacity="{}"/>"#,
                    item_x,
                    box_center_y - COLOR_BOX_SIZE / 2.0,
                    COLOR_BOX_SIZE,
                    COLOR_BOX_SIZE,
                    color,
                    opacity
                )?;
            }
        }

        // Draw label - vertically centered with the color box
        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="{}" font-family="{}" dominant-baseline="middle">{}</text>"#,
            item_x + COLOR_BOX_SIZE + COLOR_BOX_SPACING,
            box_center_y,
            theme.tick_label_font_size,
            theme.tick_label_font_family,
            value
        )?;
    }

    Ok(())
}
