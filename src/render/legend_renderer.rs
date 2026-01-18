use crate::visual::shape::PointShape;
use crate::theme::Theme;
use crate::core::layer::RenderBackend;
use super::backend::svg::SvgBackend;
use crate::core::legend::{LegendSpec, LegendPosition, LegendSize};
use crate::core::context::SharedRenderingContext;
use crate::scale::ScaleDomain;

/// LegendRenderer is responsible for drawing visual guides (legends) that explain 
/// the scales (color, size, shape) used in the plot.
pub struct LegendRenderer;

impl LegendRenderer {
    /// The main entry point for rendering the legend.
    /// It handles the high-level layout, including wrapping legend blocks based on 
    /// the available panel space and the chosen legend position.
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
        
        // Orientation depends on the position (Top/Bottom are horizontal, Left/Right vertical)
        let is_horizontal = matches!(ctx.legend_position, LegendPosition::Top | LegendPosition::Bottom);

        // Determine the starting coordinate for the first legend block
        let (start_x, start_y) = Self::calculate_initial_anchor(ctx, specs, theme, is_horizontal);

        let mut current_x = start_x;
        let mut current_y = start_y;
        let mut max_dim_in_row_col = 0.0; 
        
        let block_gap = theme.legend_block_gap;
        let plot_limit_h = ctx.panel.height;
        let plot_limit_w = ctx.panel.width;

        for spec in specs {
            // Estimate size using 150px as default wrap width for horizontal layouts
            let block_size = spec.estimate_size(theme, if is_horizontal { 150.0 } else { plot_limit_h });

            // --- MACRO-LAYOUT WRAPPING ---
            if !is_horizontal {
                // Vertical layout: Wrap to a new column if we exceed panel height
                if current_y + block_size.height > start_y + plot_limit_h && current_y > start_y {
                    current_x += max_dim_in_row_col + block_gap;
                    current_y = start_y;
                    max_dim_in_row_col = block_size.width;
                } else {
                    max_dim_in_row_col = f64::max(max_dim_in_row_col, block_size.width);
                }
            } else {
                // Horizontal layout: Wrap to a new row if we exceed panel width
                if current_x + block_size.width > start_x + plot_limit_w && current_x > start_x {
                    current_y += max_dim_in_row_col + block_gap;
                    current_x = start_x;
                    max_dim_in_row_col = block_size.height;
                } else {
                    max_dim_in_row_col = f64::max(max_dim_in_row_col, block_size.height);
                }
            }

            // 1. Draw the Legend Title
            backend.draw_text(
                &spec.title,
                current_x,
                current_y + (font_size * 0.8),
                font_size * 1.1,
                font_family,
                &theme.title_color,
                "start",
                "bold",
                1.0,
            );

            // 2. Resolve data values into visual properties (colors, shapes, radii)
            let (labels, colors, shapes, sizes) = Self::resolve_mappings(spec, ctx);

            // 3. Draw the items (Glyphs + Text Labels)
            let actual_block_size = Self::draw_spec_group(
                &mut backend,
                spec,
                &labels,
                &colors,
                shapes.as_deref(),
                sizes.as_deref(), 
                current_x,
                current_y + (font_size * 1.1) + theme.legend_title_gap,
                font_size,
                theme,
                if is_horizontal { 150.0 } else { plot_limit_h }
            );

            // 4. Update cursor position for the next legend block
            if !is_horizontal {
                current_y += actual_block_size.height + block_gap;
            } else {
                current_x += actual_block_size.width + block_gap;
            }
        }
    }

    /// Resolves data labels into specific visual properties by querying the scales.
    fn resolve_mappings(
        spec: &LegendSpec,
        ctx: &SharedRenderingContext,
    ) -> (Vec<String>, Vec<String>, Option<Vec<PointShape>>, Option<Vec<f64>>) {
        let labels = match &spec.domain {
            ScaleDomain::Categorical(values) => values.clone(),
            _ => spec.get_sampling_labels(),
        };

        let mut colors = Vec::new();
        let mut shapes = Vec::new();
        let mut sizes = Vec::new();

        for val_str in &labels {
            // A. Color Mapping
            if spec.has_color {
                if let Some((scale, mapper)) = &ctx.aesthetics.color {
                    let norm = scale.normalize_string(val_str);
                    colors.push(mapper.map_to_color(norm, scale.logical_max()));
                }
            } else {
                colors.push("#333333".into());
            }

            // B. Shape Mapping
            if spec.has_shape {
                if let Some((scale, mapper)) = &ctx.aesthetics.shape {
                    let norm = scale.normalize_string(val_str);
                    shapes.push(mapper.map_to_shape(norm, scale.logical_max()));
                }
            } else {
                shapes.push(PointShape::Circle);
            }

            // C. Size Mapping (Radius)
            if spec.has_size {
                if let Some((scale, mapper)) = &ctx.aesthetics.size {
                    match &spec.domain {
                        ScaleDomain::Categorical(_) => {
                            let norm = scale.normalize_string(val_str);
                            sizes.push(mapper.map_to_size(norm));
                        }
                        _ => {
                            if let Ok(val_num) = val_str.parse::<f64>() {
                                let norm = scale.normalize(val_num);
                                sizes.push(mapper.map_to_size(norm));
                            } else {
                                sizes.push(5.0);
                            }
                        }
                    }
                }
            } else {
                sizes.push(5.0);
            }
        }

        (labels, colors, if spec.has_shape { Some(shapes) } else { None }, if spec.has_size { Some(sizes) } else { None })
    }

    /// Renders a group of items (symbols and text) for a single legend block.
    fn draw_spec_group(
        backend: &mut dyn RenderBackend,
        _spec: &LegendSpec,
        labels: &[String],
        colors: &[String],
        shapes: Option<&[PointShape]>,
        sizes: Option<&[f64]>,
        x: f64,
        y: f64,
        font_size: f64,
        theme: &Theme,
        max_h: f64,
    ) -> LegendSize {
        let mut col_x = x;
        let mut item_y = y;
        let mut current_col_w = 0.0;
        let mut total_w = 0.0;
        
        let font_family = theme.legend_font_family.as_ref().unwrap_or(&theme.tick_label_font_family);
        let item_v_gap = theme.legend_item_v_gap;
        let col_h_gap = theme.legend_col_h_gap;
        let marker_to_text_gap = theme.legend_marker_text_gap;

        // --- FIXED DIMENSIONS ---
        // We use 18.0px to match estimate_size in legend.rs.
        // This accommodates radius values up to 9.0px (diameter 18.0px).
        let fixed_container_size = 18.0; 

        for (i, label) in labels.iter().enumerate() {
            // --- RADIUS RETRIEVAL ---
            // Directly use the raw radius from the mapper. With mapper range (2.0, 8.0),
            // all steps will be clearly distinct and fit inside the 18px container.
            let r = sizes
                .and_then(|s| s.get(i))
                .cloned()
                .unwrap_or(5.0);

            let text_w = crate::core::utils::estimate_text_width(label, font_size);
            let row_w = fixed_container_size + marker_to_text_gap + text_w;
            let row_h = f64::max(fixed_container_size, font_size);

            // Internal Column Wrapping Logic
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

            // Draw Symbol (Solid Fill)
            Self::draw_symbol(
                backend, 
                shape, 
                col_x + (fixed_container_size / 2.0), 
                item_y + (row_h / 2.0), 
                r, 
                color
            );

            // Draw Label Text
            backend.draw_text(
                label,
                col_x + fixed_container_size + marker_to_text_gap,
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

        LegendSize {
            width: total_w + current_col_w,
            height: if total_w > 0.0 { max_h } else { item_y - y },
        }
    }

    /// Renders the geometric shape for the legend marker with SOLID FILL.
    fn draw_symbol(backend: &mut dyn RenderBackend, shape: &PointShape, cx: f64, cy: f64, r: f64, color: &str) {
        // We use Some(color) for the fill parameter to create solid-filled markers.
        match shape {
            PointShape::Circle => {
                backend.draw_circle(cx, cy, r, Some(color), None, 0.0, 1.0)
            },
            PointShape::Square => {
                backend.draw_rect(cx - r, cy - r, r * 2.0, r * 2.0, Some(color), None, 0.0, 1.0)
            },
            PointShape::Triangle => {
                let pts = vec![(cx, cy - r), (cx - r, cy + r), (cx + r, cy + r)];
                backend.draw_polygon(&pts, Some(color), None, 0.0, 1.0);
            },
            PointShape::Diamond => {
                let pts = vec![(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)];
                backend.draw_polygon(&pts, Some(color), None, 0.0, 1.0);
            },
            _ => backend.draw_circle(cx, cy, r, Some(color), None, 0.0, 1.0),
        }
    }

    /// Calculates where the legend drawing should begin based on position and margins.
    fn calculate_initial_anchor(ctx: &SharedRenderingContext, _: &[LegendSpec], theme: &Theme, _: bool) -> (f64, f64) {
        let mut x = ctx.panel.x;
        let mut y = ctx.panel.y;
        match ctx.legend_position {
            LegendPosition::Right => x = ctx.panel.x + ctx.panel.width + ctx.legend_margin,
            LegendPosition::Left => x = (ctx.panel.x - ctx.legend_margin - theme.axis_reserve_buffer).max(10.0),
            LegendPosition::Top => y = (ctx.panel.y - ctx.legend_margin - (theme.axis_reserve_buffer * 0.8)).max(10.0),
            LegendPosition::Bottom => y = ctx.panel.y + ctx.panel.height + ctx.legend_margin,
            _ => {}
        }
        (x, y)
    }
}