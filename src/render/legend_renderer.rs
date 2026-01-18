use crate::visual::shape::PointShape;
use crate::theme::Theme;
use crate::core::layer::RenderBackend;
use super::backend::svg::SvgBackend;
use crate::core::legend::{LegendSpec, LegendPosition};
use crate::core::context::SharedRenderingContext;
use crate::scale::ScaleDomain;
use std::f64::consts::PI;

/// LegendRenderer is responsible for drawing visual guides that explain the scales used in the plot.
/// 
/// It implements the "Macro-Layout" strategy:
/// 1. Vertical (Left/Right): Blocks stack vertically until they hit the plot height, then wrap to new columns.
/// 2. Horizontal (Top/Bottom): Blocks stack horizontally until they hit the plot width, then wrap to new rows.
pub struct LegendRenderer;

impl LegendRenderer {
    /// The main entry point for the legend rendering process.
    pub fn render_legend(
        buffer: &mut String,
        specs: &[LegendSpec],
        theme: &Theme,
        ctx: &SharedRenderingContext,
    ) {
        if specs.is_empty() || matches!(ctx.legend_position, LegendPosition::None) {
            return;
        }

        let mut backend = SvgBackend::new(buffer, None);

        let font_size = theme.legend_font_size.unwrap_or(theme.tick_label_font_size);
        let font_family = theme.legend_font_family.as_ref().unwrap_or(&theme.tick_label_font_family);
        
        // Determine layout orientation based on the strategic position.
        let is_horizontal = matches!(ctx.legend_position, LegendPosition::Top | LegendPosition::Bottom);

        // Calculate the starting anchor point (top-left of the first legend block).
        let (start_x, start_y) = Self::calculate_initial_anchor(ctx, specs, theme, is_horizontal);

        // Cursor tracking for wrapping logic.
        let mut current_x = start_x;
        let mut current_y = start_y;
        let mut max_dim_in_row_col = 0.0; // Tracks col_width (vertical) or row_height (horizontal)
        
        // Use theme-driven gap between distinct legend blocks.
        let block_gap = theme.legend_block_gap;
        let plot_limit_h = ctx.panel.height;
        let plot_limit_w = ctx.panel.width;

        for spec in specs {
            // Measure the individual block size using theme-aware logic.
            // Horizontal legends are capped at 150px height to prevent crushing the plot.
            let block_size = spec.estimate_size(theme, if is_horizontal { 150.0 } else { plot_limit_h });

            // --- WRAPPING LOGIC (Macro-Layout Replay) ---
            // We replay the logic used in the LayoutEngine to ensure drawing aligns with reserved space.
            if !is_horizontal {
                // Vertical Placement: Check for Y-overflow to wrap to a new column.
                if current_y + block_size.height > start_y + plot_limit_h && current_y > start_y {
                    current_x += max_dim_in_row_col + block_gap;
                    current_y = start_y;
                    max_dim_in_row_col = block_size.width;
                } else {
                    max_dim_in_row_col = f64::max(max_dim_in_row_col, block_size.width);
                }
            } else {
                // Horizontal Placement: Check for X-overflow to wrap to a new row.
                if current_x + block_size.width > start_x + plot_limit_w && current_x > start_x {
                    current_y += max_dim_in_row_col + block_gap;
                    current_x = start_x;
                    max_dim_in_row_col = block_size.height;
                } else {
                    max_dim_in_row_col = f64::max(max_dim_in_row_col, block_size.height);
                }
            }

            // --- DRAW BLOCK CONTENT ---
            // 1. Draw Title
            backend.draw_text(
                &spec.title,
                current_x,
                current_y + (font_size * 0.8), // Align text to baseline
                font_size * 1.1,
                font_family,
                &theme.title_color,
                "start",
                "bold",
                1.0,
            );

            // 2. Resolve data-to-visual mappings (colors, shapes)
            let (labels, colors, shapes) = Self::resolve_mappings(spec, ctx);

            // 3. Draw items within this block
            // Vertical offset accounts for title height and the theme-defined title gap.
            let actual_block_size = Self::draw_spec_group(
                &mut backend,
                spec,
                &labels,
                &colors,
                shapes.as_deref(),
                current_x,
                current_y + (font_size * 1.1) + theme.legend_title_gap,
                font_size,
                theme,
                if is_horizontal { 150.0 } else { plot_limit_h }
            );

            // 4. Update cursor for the next LegendSpec block.
            if !is_horizontal {
                current_y += actual_block_size.height + block_gap;
            } else {
                current_x += actual_block_size.width + block_gap;
            }
        }
    }

    /// Calculates the starting top-left coordinate where the legend group begins.
    /// It uses the axis_reserve_buffer to ensure legends don't overlap axis labels.
    fn calculate_initial_anchor(
        ctx: &SharedRenderingContext,
        _specs: &[LegendSpec],
        theme: &Theme,
        _is_horizontal: bool,
    ) -> (f64, f64) {
        let mut x = ctx.panel.x;
        let mut y = ctx.panel.y;

        match ctx.legend_position {
            LegendPosition::Right => {
                x = ctx.panel.x + ctx.panel.width + ctx.legend_margin;
            }
            LegendPosition::Left => {
                // Buffer ensures we stay clear of the Y-axis labels.
                x = (ctx.panel.x - ctx.legend_margin - theme.axis_reserve_buffer).max(10.0); 
            }
            LegendPosition::Top => {
                // Buffer ensures we stay clear of the title or Top X-axis.
                y = (ctx.panel.y - ctx.legend_margin - (theme.axis_reserve_buffer * 0.8)).max(10.0);
            }
            LegendPosition::Bottom => {
                y = ctx.panel.y + ctx.panel.height + ctx.legend_margin;
            }
            _ => {}
        }
        (x, y)
    }

    /// Queries the VisualMappers in the SharedContext to retrieve assigned colors and shapes.
    fn resolve_mappings(
        spec: &LegendSpec,
        ctx: &SharedRenderingContext,
    ) -> (Vec<String>, Vec<String>, Option<Vec<PointShape>>) {
        let mut labels = Vec::new();
        let mut colors = Vec::new();
        let mut shapes = Vec::new();

        match &spec.domain {
            ScaleDomain::Categorical(domain_values) => {
                for val in domain_values {
                    labels.push(val.clone());
                    
                    // Resolve Color Mapping
                    if spec.has_color {
                        if let Some((scale, mapper)) = &ctx.aesthetics.color {
                            let norm = scale.normalize_string(val);
                            colors.push(mapper.map_to_color(norm, scale.logical_max()));
                        }
                    } else {
                        colors.push("#333333".into());
                    }

                    // Resolve Shape Mapping
                    if spec.has_shape {
                        if let Some((scale, mapper)) = &ctx.aesthetics.shape {
                            let norm = scale.normalize_string(val);
                            shapes.push(mapper.map_to_shape(norm, scale.logical_max()));
                        }
                    } else {
                        shapes.push(PointShape::Circle);
                    }
                }
            }
            _ => {
                // Use the 5 sample ticks generated for Continuous/Temporal domains.
                labels = spec.get_sampling_labels();
                for _ in 0..labels.len() { colors.push("#555555".into()); }
            }
        }
        (labels, colors, if spec.has_shape { Some(shapes) } else { None })
    }

    /// Renders items inside a single LegendSpec block.
    /// Handles internal column wrapping if items exceed the available plot height.
    fn draw_spec_group(
        backend: &mut dyn RenderBackend,
        _spec: &LegendSpec,
        labels: &[String],
        colors: &[String],
        shapes: Option<&[PointShape]>,
        x: f64,
        y: f64,
        font_size: f64,
        theme: &Theme,
        max_h: f64,
    ) -> crate::core::legend::LegendSize {
        let mut col_x = x;
        let mut item_y = y;
        let mut current_col_w = 0.0;
        let mut total_w = 0.0;
        
        let font_family = theme.legend_font_family.as_ref().unwrap_or(&theme.tick_label_font_family);
        
        // Use theme parameters for item and column spacing.
        let item_v_gap = theme.legend_item_v_gap;
        let col_h_gap = theme.legend_col_h_gap;
        let marker_to_text_gap = theme.legend_marker_text_gap;

        for (i, label) in labels.iter().enumerate() {
            let marker_size = 12.0;
            let text_w = crate::core::utils::estimate_text_width(label, font_size);
            let row_w = marker_size + marker_to_text_gap + text_w;
            let row_h = f64::max(marker_size, font_size);

            // Internal wrapping: Start new column if this item exceeds the vertical limit.
            if item_y + row_h > y + max_h && i > 0 {
                total_w += current_col_w + col_h_gap;
                col_x += current_col_w + col_h_gap;
                item_y = y;
                current_col_w = row_w;
            } else {
                current_col_w = f64::max(current_col_w, row_w);
            }

            let color = colors.get(i).map(|s| s.as_str()).unwrap_or("#333333");
            let shape = shapes.and_then(|s| s.get(i)).unwrap_or(&PointShape::Circle);

            // Draw the Glyph (Symbol)
            Self::draw_symbol(backend, shape, col_x + (marker_size / 2.0), item_y + (row_h / 2.0), marker_size / 2.0, color);

            // Draw the Label text
            backend.draw_text(
                label,
                col_x + marker_size + marker_to_text_gap,
                item_y + (row_h * 0.75),
                font_size,
                font_family,
                &theme.legent_label_color,
                "start",
                "normal",
                1.0,
            );

            item_y += row_h + item_v_gap;
        }

        crate::core::legend::LegendSize {
            width: total_w + current_col_w,
            height: if total_w > 0.0 { max_h } else { item_y - y },
        }
    }

    /// Renders specific geometric paths (Circle, Square, etc.) for the legend markers.
    fn draw_symbol(backend: &mut dyn RenderBackend, shape: &PointShape, cx: f64, cy: f64, r: f64, color: &str) {
        match shape {
            PointShape::Circle => backend.draw_circle(cx, cy, r, Some(color), None, 0.0, 1.0),
            PointShape::Square => backend.draw_rect(cx - r, cy - r, r * 2.0, r * 2.0, Some(color), None, 0.0, 1.0),
            PointShape::Triangle => {
                let pts = vec![(cx, cy - r), (cx - r, cy + r), (cx + r, cy + r)];
                backend.draw_polygon(&pts, Some(color), None, 0.0, 1.0);
            }
            PointShape::Diamond => {
                let pts = vec![(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)];
                backend.draw_polygon(&pts, Some(color), None, 0.0, 1.0);
            }
            PointShape::Star => {
                let mut pts = Vec::with_capacity(10);
                for i in 0..10 {
                    let angle = (i as f64) * PI / 5.0 - PI / 2.0;
                    let radius = if i % 2 == 0 { r } else { r * 0.45 };
                    pts.push((cx + radius * angle.cos(), cy + radius * angle.sin()));
                }
                backend.draw_polygon(&pts, Some(color), None, 0.0, 1.0);
            }
            _ => backend.draw_circle(cx, cy, r, Some(color), None, 0.0, 1.0),
        }
    }
}