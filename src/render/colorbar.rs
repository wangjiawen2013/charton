use crate::core::context::SharedRenderingContext;
use crate::core::layer::RenderBackend;
use crate::theme::Theme;
use crate::core::legend::{LegendSpec, LegendPosition};

/// Renders a continuous color scale guide (Colorbar) with full orientation awareness.
/// 
/// The bar's dimensions are derived from the `total_block_dim` provided by the 
/// layout engine, ensuring it fits perfectly within the allocated legend area.
pub(crate) fn render_colorbar(
    backend: &mut dyn RenderBackend,
    spec: &LegendSpec,
    theme: &Theme,
    context: &SharedRenderingContext,
    x: f64,
    y: f64,
    total_block_dim: f64, // Width if horizontal, Height if vertical
) {
    let (color_scale, visual_mapper) = match &context.aesthetics.color {
        Some((s, m)) => (s, m),
        None => return,
    };

    let is_horizontal = matches!(context.legend_position, LegendPosition::Top | LegendPosition::Bottom);
    let font_size = theme.legend_label_size.unwrap_or(theme.tick_label_size);
    let font_family = theme.legend_label_family.as_deref().unwrap_or(&theme.label_family);
    
    let title_h = font_size * 1.1;
    let title_gap = theme.legend_title_gap;
    let marker_gap = theme.legend_marker_text_gap;

    // 1. Render the Title (Always at the top-left of the block)
    backend.draw_text(
        &spec.title,
        x,
        y + (font_size * 0.8),
        title_h,
        font_family,
        &theme.title_color,
        "start",
        "bold",
        1.0,
    );

    let steps = 32;
    let l_max = color_scale.logical_max();
    let bar_start_y = y + title_h + title_gap;

    if is_horizontal {
        // --- ADAPTIVE HORIZONTAL LAYOUT ---
        // Width is determined by the layout engine's allocation for this block.
        let bar_w = total_block_dim;
        let bar_h = 12.0; // Horizontal bars are typically thinner
        let step_w = bar_w / steps as f64;

        // Draw gradient rectangles (Left to Right)
        for i in 0..steps {
            let norm_val = i as f64 / steps as f64;
            let color = visual_mapper.map_to_color(norm_val, l_max);
            backend.draw_rect(
                x + (i as f64 * step_w),
                bar_start_y,
                step_w + 0.5, // Overlap to prevent sub-pixel gaps
                bar_h,
                Some(&color),
                None,
                0.0,
                1.0,
            );
        }

        // Render Ticks and Labels (Below the bar)
        let ticks = color_scale.ticks(5);
        for tick in ticks {
            let norm = color_scale.normalize(tick.value);
            let tick_x = x + norm * bar_w;
            
            // White tick line inside the bar
            backend.draw_path(
                &[(tick_x, bar_start_y + bar_h - 4.0), (tick_x, bar_start_y + bar_h)],
                "white",
                1.0,
                0.8,
            );

            // Centered label below
            backend.draw_text(
                &tick.label,
                tick_x,
                bar_start_y + bar_h + marker_gap + font_size,
                font_size,
                font_family,
                &theme.legend_label_color,
                "middle",
                "normal",
                1.0,
            );
        }
    } else {
        // --- ADAPTIVE VERTICAL LAYOUT ---
        // Height is determined by the layout engine (e.g., min(200, panel_h * 0.8)).
        let bar_w = 15.0;
        let bar_h = (total_block_dim - title_h - title_gap).max(10.0);
        let step_h = bar_h / steps as f64;

        // Draw gradient rectangles (Top to Bottom)
        for i in 0..steps {
            // In vertical, highest value (1.0) is at the top (i=0)
            let norm_val = 1.0 - (i as f64 / steps as f64);
            let color = visual_mapper.map_to_color(norm_val, l_max);
            backend.draw_rect(
                x,
                bar_start_y + (i as f64 * step_h),
                bar_w,
                step_h + 0.5,
                Some(&color),
                None,
                0.0,
                1.0,
            );
        }

        // Render Ticks and Labels (To the right)
        let ticks = color_scale.ticks(5);
        for tick in ticks {
            let norm = color_scale.normalize(tick.value);
            let tick_y = bar_start_y + (1.0 - norm) * bar_h;

            // White tick line inside the bar
            backend.draw_path(
                &[(x + bar_w - 4.0, tick_y), (x + bar_w, tick_y)],
                "white",
                1.0,
                0.6,
            );

            // Label to the right
            backend.draw_text(
                &tick.label,
                x + bar_w + marker_gap,
                tick_y + (font_size * 0.35), // Vertical centering
                font_size,
                font_family,
                &theme.legend_label_color,
                "start",
                "normal",
                1.0,
            );
        }
    }
}