use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;
use crate::theme::Theme;
use crate::core::layer::RenderBackend;
use super::backend::svg::SvgBackend;
use crate::core::guide::{GuideSpec, LegendPosition, GuideSize, GuideKind};
use crate::core::context::SharedRenderingContext;
use crate::scale::ScaleDomain;
use crate::scale::mapper::VisualMapper;

/// LegendRenderer translates abstract GuideSpecs into visual SVG representations.
/// It handles both discrete "Legends" (categories) and continuous "ColorBars" (gradients).
pub struct LegendRenderer;

impl LegendRenderer {
    /// The primary entry point for rendering the legend area.
    /// Handles macro-layout, including block wrapping and positioning relative to the plot panel.
    pub fn render_legend(
        buffer: &mut String,
        specs: &[GuideSpec],
        theme: &Theme,
        ctx: &SharedRenderingContext,
    ) {
        if specs.is_empty() || matches!(ctx.legend_position, LegendPosition::None) {
            return;
        }

        let mut backend = SvgBackend::new(buffer, None);
        let font_size = theme.legend_label_size;
        let font_family = &theme.legend_label_family;
        
        // Layout orientation: Top/Bottom positions are horizontal; Left/Right are vertical.
        let is_horizontal = matches!(ctx.legend_position, LegendPosition::Top | LegendPosition::Bottom);

        // Determine starting coordinates based on margins and axis buffers.
        let (start_x, start_y) = Self::calculate_initial_anchor(ctx, specs, theme, is_horizontal);

        let mut current_x = start_x;
        let mut current_y = start_y;
        let mut max_dim_in_row_col = 0.0; 
        let block_gap = theme.legend_block_gap;

        for spec in specs {
            // Estimate size for layout calculation.
            let block_size = spec.estimate_size(theme, if is_horizontal { 150.0 } else { ctx.panel.height });

            // --- Macro-Layout Wrapping ---
            if !is_horizontal {
                // Vertical layout: wrap to a new column if panel height is exceeded.
                if current_y + block_size.height > start_y + ctx.panel.height && current_y > start_y {
                    current_x += max_dim_in_row_col + block_gap;
                    current_y = start_y;
                    max_dim_in_row_col = block_size.width;
                } else {
                    max_dim_in_row_col = f32::max(max_dim_in_row_col, block_size.width);
                }
            } else {
                // Horizontal layout: wrap to a new row if panel width is exceeded.
                if current_x + block_size.width > start_x + ctx.panel.width && current_x > start_x {
                    current_y += max_dim_in_row_col + block_gap;
                    current_x = start_x;
                    max_dim_in_row_col = block_size.height;
                } else {
                    max_dim_in_row_col = f32::max(max_dim_in_row_col, block_size.height);
                }
            }

            // 1. Draw Title
            backend.draw_text(
                &spec.title,
                current_x,
                current_y + (font_size * 0.8),
                font_size * 1.1,
                font_family,
                &theme.title_color,
                "start", "bold", 1.0,
            );

            let content_y_offset = current_y + (font_size * 1.1) + theme.legend_title_gap;

            // 2. Render content based on GuideKind
            let actual_block_size = match spec.kind {
                GuideKind::ColorBar => {
                    Self::draw_colorbar(&mut backend, spec, ctx, current_x, content_y_offset, theme)
                },
                GuideKind::Legend => {
                    let (labels, colors, shapes, sizes) = Self::resolve_mappings(spec, ctx);
                    Self::draw_spec_group(
                        &mut backend, spec, &labels, &colors, shapes.as_deref(), sizes.as_deref(),
                        current_x, content_y_offset, font_size, theme,
                        if is_horizontal { 150.0 } else { ctx.panel.height }
                    )
                }
            };

            // 3. Advance cursor for the next guide block
            if !is_horizontal {
                current_y += actual_block_size.height + block_gap;
            } else {
                current_x += actual_block_size.width + block_gap;
            }
        }
    }

    /// Renders a continuous color gradient bar with associated ticks and labels.
    fn draw_colorbar(
        backend: &mut dyn RenderBackend,
        spec: &GuideSpec,
        ctx: &SharedRenderingContext,
        x: f32,
        y: f32,
        theme: &Theme,
    ) -> GuideSize {
        let bar_w = 15.0;
        let bar_h = 150.0; 
        let font_size = theme.legend_label_size;
        let font_family = &theme.legend_label_family;

        // A. Sample Gradient Stops
        let mut stops = Vec::new();

        // Safely access the color mapping from the aesthetics context
        if let Some(ref mapping) = ctx.aesthetics.color {
            // Access the mapper via the scale implementation's trait method.
            // This maintains the single source of truth for visual transformations.
            if let Some(mapper) = mapping.scale_impl.mapper() {
                let n_samples = 15;

                // Retrieve the logical maximum (e.g., number of categories - 1 for discrete scales)
                // to ensure correct color indexing during sampling.
                let l_max = mapping.scale_impl.logical_max();

                for i in 0..=n_samples {
                    // Calculate the stop position ratio [0.0, 1.0]
                    let ratio = i as f32 / n_samples as f32;

                    // Reverse sampling (1.0 - ratio) ensures that high data values 
                    // are mapped to the top of the visual color bar.
                    let color = mapper.map_to_color(1.0 - ratio, l_max);

                    stops.push((ratio, color));
                }
            }
        }

        // B. Render Color Strip
        backend.draw_gradient_rect(x, y, bar_w, bar_h, &stops, true, &spec.field);
        backend.draw_rect(x, y, bar_w, bar_h, &SingleColor::new("none"), &theme.title_color, 1.0, 1.0);

        // C. Render Ticks and Labels
        let mut max_label_w = 0.0;
        if let Some(mapping) = spec.mappings.first() {
            let ticks = mapping.scale_impl.ticks(5);
            for tick in ticks {
                let norm = mapping.scale_impl.normalize(tick.value);
                let tick_y = y + (bar_h * (1.0 - norm));

                // Interior tick markers (White for contrast against the gradient)
                backend.draw_line(x, tick_y, x + 3.0, tick_y, &"#FFFFFF".into(), 1.0);
                backend.draw_line(x + bar_w - 3.0, tick_y, x + bar_w, tick_y, &"#FFFFFF".into(), 1.0);

                backend.draw_text(
                    &tick.label,
                    x + bar_w + theme.legend_marker_text_gap,
                    tick_y + (font_size * 0.3),
                    font_size,
                    font_family,
                    &theme.legend_label_color,
                    "start", "normal", 1.0,
                );
                
                let lw = crate::core::utils::estimate_text_width(&tick.label, font_size);
                max_label_w = f32::max(max_label_w, lw);
            }
        }

        GuideSize {
            width: bar_w + theme.legend_marker_text_gap + max_label_w,
            height: bar_h,
        }
    }

    /// Renders a discrete legend group (categorical markers + text).
    fn draw_spec_group(
        backend: &mut dyn RenderBackend,
        _spec: &GuideSpec,
        labels: &[String],
        colors: &[SingleColor],
        shapes: Option<&[PointShape]>,
        sizes: Option<&[f32]>,
        x: f32,
        y: f32,
        font_size: f32,
        theme: &Theme,
        max_h: f32,
    ) -> GuideSize {
        let mut col_x = x;
        let mut item_y = y;
        let mut current_col_w = 0.0;
        let mut total_w = 0.0;
        
        let font_family = &theme.legend_label_family;
        let fixed_container_size = 18.0; 

        for (i, label) in labels.iter().enumerate() {
            let r = sizes.and_then(|s| s.get(i)).cloned().unwrap_or(5.0);
            let text_w = crate::core::utils::estimate_text_width(label, font_size);
            let row_w = fixed_container_size + theme.legend_marker_text_gap + text_w;
            let row_h = f32::max(fixed_container_size, font_size);

            // Column Wrapping within a block
            if item_y + row_h > y + max_h && i > 0 {
                total_w += current_col_w + theme.legend_col_h_gap;
                col_x += current_col_w + theme.legend_col_h_gap;
                item_y = y;
                current_col_w = row_w;
            } else {
                current_col_w = f32::max(current_col_w, row_w);
            }

            let shape = shapes.and_then(|s| s.get(i)).unwrap_or(&PointShape::Circle);

            Self::draw_symbol(
                backend, shape, 
                col_x + (fixed_container_size / 2.0), 
                item_y + (row_h / 2.0), r, colors.get(i).unwrap_or(&"#333333".into())
            );

            backend.draw_text(
                label,
                col_x + fixed_container_size + theme.legend_marker_text_gap,
                item_y + (row_h * 0.75),
                font_size, font_family, &theme.legend_label_color,
                "start", "normal", 1.0,
            );

            item_y += row_h + theme.legend_item_v_gap;
        }

        GuideSize {
            width: total_w + current_col_w,
            height: if total_w > 0.0 { max_h } else { item_y - y },
        }
    }

    /// Resolves data samples into physical visual properties by querying GlobalAesthetics.
    /// 
    /// This method maps data values to visual attributes (Color, Shape, Size). 
    /// For continuous scales, it utilizes the raw numeric values from Ticks to ensure 
    /// precision, avoiding the fragility of parsing formatted strings back into floats.
    fn resolve_mappings(
        spec: &GuideSpec,
        ctx: &SharedRenderingContext,
    ) -> (Vec<String>, Vec<SingleColor>, Option<Vec<PointShape>>, Option<Vec<f32>>) {
        // 1. Retrieve Ticks instead of just strings for non-categorical domains.
        // This gives us access to both 'tick.value' (for mapping) and 'tick.label' (for display).
        let (labels, values_f32): (Vec<String>, Vec<f32>) = match &spec.domain {
            ScaleDomain::Discrete(values) => {
                // For categories, value and label are treated as the same string.
                (values.clone(), Vec::new())
            }
            _ => {
                // For Continuous/Log/Temporal, we fetch synced ticks.
                // Note: We need a small helper or to modify GuideSpec to return Ticks here.
                // Assuming get_sampling_ticks() returns Vec<Tick> with aligned labels.
                let ticks = spec.get_sampling_ticks(); 
                let l = ticks.iter().map(|t| t.label.clone()).collect();
                let v = ticks.iter().map(|t| t.value).collect();
                (l, v)
            }
        };

        let mut colors = Vec::new();
        let mut shapes = Vec::new();
        let mut sizes = Vec::new();

        let has_color = spec.mappings.iter().any(|m| {
            // Access the mapper through the scale implementation.
            // If a scale doesn't have a mapper, it cannot contribute to color rendering.
            if let Some(mapper) = m.scale_impl.mapper() {
                matches!(
                    mapper, 
                    VisualMapper::DiscreteColor { .. } | VisualMapper::ContinuousColor { .. }
                )
            } else {
                false
            }
        });

        // Check if any mapping involves a geometric shape mapper
        let has_shape = spec.mappings.iter().any(|m| {
            m.scale_impl.mapper().map_or(false, |v| matches!(v, VisualMapper::Shape { .. }))
        });

        // Check if any mapping involves a physical size mapper
        let has_size = spec.mappings.iter().any(|m| {
            m.scale_impl.mapper().map_or(false, |v| matches!(v, VisualMapper::Size { .. }))
        });

        for (i, label_str) in labels.iter().enumerate() {
            // 2. Determine the numeric value for this sample if applicable.
            let val_f32 = values_f32.get(i).cloned();

            // 3. Resolve Color
            if has_color {
                if let Some(ref mapping) = ctx.aesthetics.color {
                    // Get the normalized value [0, 1] from the scale
                    let norm = val_f32.map(|v| mapping.scale_impl.normalize(v))
                        .unwrap_or_else(|| mapping.scale_impl.normalize_string(label_str));

                    // Access the mapper via the scale implementation
                    let color = mapping.scale_impl.mapper()
                        .map(|m| m.map_to_color(norm, mapping.scale_impl.logical_max()))
                        .unwrap_or_else(|| "#333333".into());
                    colors.push(color);
                }
            } else { colors.push("#333333".into()); }

            // 4. Resolve Shape
            if has_shape {
                if let Some(ref mapping) = ctx.aesthetics.shape {
                    let norm = val_f32.map(|v| mapping.scale_impl.normalize(v))
                        .unwrap_or_else(|| mapping.scale_impl.normalize_string(label_str));

                    let shape = mapping.scale_impl.mapper()
                        .map(|m| m.map_to_shape(norm, mapping.scale_impl.logical_max()))
                        .unwrap_or(PointShape::Circle);
                    shapes.push(shape);
                }
            } else { shapes.push(PointShape::Circle); }

            // 5. Resolve Size
            if has_size {
                if let Some(ref mapping) = ctx.aesthetics.size {
                    let norm = val_f32.map(|v| mapping.scale_impl.normalize(v))
                        .unwrap_or_else(|| mapping.scale_impl.normalize_string(label_str));

                    let size = mapping.scale_impl.mapper()
                        .map(|m| m.map_to_size(norm))
                        .unwrap_or(5.0);
                    sizes.push(size);
                }
            } else { sizes.push(5.0); }
        }

        (labels, colors, if has_shape { Some(shapes) } else { None }, if has_size { Some(sizes) } else { None })
    }

    fn draw_symbol(backend: &mut dyn RenderBackend, shape: &PointShape, cx: f32, cy: f32, r: f32, color: &SingleColor) {
        match shape {
            PointShape::Circle => backend.draw_circle(cx, cy, r, color, &SingleColor::new("none"), 0.0, 1.0),
            PointShape::Square => backend.draw_rect(cx - r, cy - r, r * 2.0, r * 2.0, color, &SingleColor::new("none"), 0.0, 1.0),
            PointShape::Triangle => {
                let pts = vec![(cx, cy - r), (cx - r, cy + r), (cx + r, cy + r)];
                backend.draw_polygon(&pts, color, &SingleColor::new("none"), 0.0, 1.0);
            },
            PointShape::Diamond => {
                let pts = vec![(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)];
                backend.draw_polygon(&pts, color, &SingleColor::new("none"), 0.0, 1.0);
            },
            _ => backend.draw_circle(cx, cy, r, color, &SingleColor::new("none"), 0.0, 1.0),
        }
    }

    fn calculate_initial_anchor(ctx: &SharedRenderingContext, _: &[GuideSpec], theme: &Theme, _: bool) -> (f32, f32) {
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