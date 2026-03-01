use super::backend::svg::SvgBackend;
use crate::Precision;
use crate::core::context::PanelContext;
use crate::core::guide::{GuideKind, GuideSize, GuideSpec, LegendPosition};
use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PolygonConfig, RectConfig, RenderBackend,
    TextConfig,
};
use crate::scale::ScaleDomain;
use crate::scale::mapper::VisualMapper;
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

/// LegendRenderer translates abstract GuideSpecs into visual SVG representations.
///
/// It operates globally on the chart canvas but uses a PanelContext to anchor itself
/// relative to the primary plotting area.
pub struct LegendRenderer;

impl LegendRenderer {
    /// The primary entry point for rendering all legends and colorbars.
    ///
    /// It coordinates the layout flow (wrapping blocks) based on the available space
    /// around the provided PanelContext.
    pub fn render_legend(
        buffer: &mut String,
        specs: &[GuideSpec],
        theme: &Theme,
        ctx: &PanelContext,
    ) {
        // Resolve the legend position from the theme.
        let position = theme.legend_position;

        if specs.is_empty() || matches!(position, LegendPosition::None) {
            return;
        }

        let mut backend = SvgBackend::new(buffer, None);
        let font_size = theme.legend_label_size;
        let font_family = &theme.legend_label_family;

        // Layout orientation: Top/Bottom positions are horizontal; Left/Right are vertical.
        let is_horizontal = matches!(position, LegendPosition::Top | LegendPosition::Bottom);

        // Determine starting coordinates relative to the panel's bounding box.
        let (start_x, start_y) = Self::calculate_initial_anchor(ctx, theme, is_horizontal);

        let mut current_x = start_x;
        let mut current_y = start_y;
        let mut max_dim_in_row_col = 0.0;
        let block_gap = theme.legend_block_gap;

        for spec in specs {
            // Estimate size for wrapping calculations.
            // In faceted plots, we typically use the full height of the plot area.
            let block_size = spec.estimate_size(
                theme,
                if is_horizontal {
                    150.0
                } else {
                    ctx.panel.height
                },
            );

            // --- Macro-Layout Wrapping ---
            // If the next legend block exceeds the panel's bounds, wrap to a new row/column.
            if !is_horizontal {
                if current_y + block_size.height > start_y + ctx.panel.height && current_y > start_y
                {
                    current_x += max_dim_in_row_col + block_gap;
                    current_y = start_y;
                    max_dim_in_row_col = block_size.width;
                } else {
                    max_dim_in_row_col = f64::max(max_dim_in_row_col, block_size.width);
                }
            } else if current_x + block_size.width > start_x + ctx.panel.width
                && current_x > start_x
            {
                current_y += max_dim_in_row_col + block_gap;
                current_x = start_x;
                max_dim_in_row_col = block_size.height;
            } else {
                max_dim_in_row_col = f64::max(max_dim_in_row_col, block_size.height);
            }

            // 1. Draw Legend Block Title
            let text_config = TextConfig {
                text: spec.title.clone(),
                x: current_x as Precision,
                y: (current_y + (font_size * 0.8)) as Precision,
                font_size: (font_size * 1.1) as Precision,
                font_family: font_family.clone(),
                color: theme.title_color,
                text_anchor: "start".to_string(),
                font_weight: "bold".to_string(),
                opacity: 1.0,
            };
            backend.draw_text(text_config);

            let content_y_offset = current_y + (font_size * 1.1) + theme.legend_title_gap;

            // 2. Render content based on GuideKind (Continuous Gradient vs. Discrete Symbols)
            let actual_block_size = match spec.kind {
                GuideKind::ColorBar => {
                    Self::draw_colorbar(&mut backend, spec, ctx, current_x, content_y_offset, theme)
                }
                GuideKind::Legend => {
                    let (labels, colors, shapes, sizes) = Self::resolve_mappings(spec, ctx);
                    Self::draw_spec_group(
                        &mut backend,
                        spec,
                        &labels,
                        &colors,
                        shapes.as_deref(),
                        sizes.as_deref(),
                        current_x,
                        content_y_offset,
                        font_size,
                        theme,
                        if is_horizontal {
                            150.0
                        } else {
                            ctx.panel.height
                        },
                    )
                }
            };

            // 3. Advance the cursor
            if !is_horizontal {
                current_y += actual_block_size.height + block_gap;
            } else {
                current_x += actual_block_size.width + block_gap;
            }
        }
    }

    /// Renders a continuous color gradient bar (ColorBar).
    fn draw_colorbar(
        backend: &mut dyn RenderBackend,
        spec: &GuideSpec,
        ctx: &PanelContext,
        x: f64,
        y: f64,
        theme: &Theme,
    ) -> GuideSize {
        let bar_w = 15.0;
        let bar_h = 150.0;
        let font_size = theme.legend_label_size;
        let font_family = &theme.legend_label_family;

        let mut stops = Vec::new();

        // Access the color aesthetics from the central spec
        if let Some(ref mapping) = ctx.spec.aesthetics.color
            && let Some(mapper) = mapping.scale_impl.mapper()
        {
            let n_samples = 15;
            let l_max = mapping.scale_impl.logical_max();

            for i in 0..=n_samples {
                let ratio = i as f64 / n_samples as f64;
                // Reverse sampling so higher values appear at the top.
                let color = mapper.map_to_color(1.0 - ratio, l_max);
                stops.push((ratio as Precision, color));
            }
        }

        let gradient_rect_config = GradientRectConfig {
            x: x as Precision,
            y: y as Precision,
            width: bar_w as Precision,
            height: bar_h as Precision,
            stops,
            is_vertical: true,
            id_suffix: spec.field.clone(),
        };
        backend.draw_gradient_rect(gradient_rect_config);

        let rect_config = RectConfig {
            x: x as Precision,
            y: y as Precision,
            width: bar_w as Precision,
            height: bar_h as Precision,
            fill: SingleColor::new("none"),
            stroke: theme.title_color,
            stroke_width: 1.0,
            opacity: 1.0,
        };
        backend.draw_rect(rect_config);

        let mut max_label_w = 0.0;
        if let Some(mapping) = spec.mappings.first() {
            let ticks = mapping.scale_impl.ticks(5);
            for tick in ticks {
                let norm = mapping.scale_impl.normalize(tick.value);
                let tick_y = y + (bar_h * (1.0 - norm));

                let line_config = LineConfig {
                    x1: x as Precision,
                    y1: tick_y as Precision,
                    x2: (x + 3.0) as Precision,
                    y2: tick_y as Precision,
                    color: "#FFFFFF".into(),
                    width: 1.0,
                    opacity: 1.0,
                    dash: None,
                };
                backend.draw_line(line_config);

                let line_config = LineConfig {
                    x1: (x + bar_w - 3.0) as Precision,
                    y1: tick_y as Precision,
                    x2: (x + bar_w) as Precision,
                    y2: tick_y as Precision,
                    color: "#FFFFFF".into(),
                    width: 1.0,
                    opacity: 1.0,
                    dash: None,
                };
                backend.draw_line(line_config);

                let text_config = TextConfig {
                    text: tick.label.clone(),
                    x: (x + bar_w + theme.legend_marker_text_gap) as Precision,
                    y: (tick_y + font_size * 0.3) as Precision,
                    font_size: font_size as Precision,
                    font_family: font_family.clone(),
                    color: theme.legend_label_color,
                    text_anchor: "start".to_string(),
                    font_weight: "normal".to_string(),
                    opacity: 1.0,
                };
                backend.draw_text(text_config);

                let lw = crate::core::utils::estimate_text_width(&tick.label, font_size);
                max_label_w = f64::max(max_label_w, lw);
            }
        }

        GuideSize {
            width: bar_w + theme.legend_marker_text_gap + max_label_w,
            height: bar_h,
        }
    }

    /// Renders a group of categorical symbols and labels.
    #[allow(clippy::too_many_arguments)]
    fn draw_spec_group(
        backend: &mut dyn RenderBackend,
        _spec: &GuideSpec,
        labels: &[String],
        colors: &[SingleColor],
        shapes: Option<&[PointShape]>,
        sizes: Option<&[f64]>,
        x: f64,
        y: f64,
        font_size: f64,
        theme: &Theme,
        max_h: f64,
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
            let row_h = f64::max(fixed_container_size, font_size);

            if item_y + row_h > y + max_h && i > 0 {
                total_w += current_col_w + theme.legend_col_h_gap;
                col_x += current_col_w + theme.legend_col_h_gap;
                item_y = y;
                current_col_w = row_w;
            } else {
                current_col_w = f64::max(current_col_w, row_w);
            }

            let shape = shapes.and_then(|s| s.get(i)).unwrap_or(&PointShape::Circle);

            Self::draw_symbol(
                backend,
                shape,
                col_x + (fixed_container_size / 2.0),
                item_y + (row_h / 2.0),
                r,
                colors.get(i).unwrap_or(&"#333333".into()),
            );

            let text_config = TextConfig {
                text: label.clone(),
                x: (col_x + fixed_container_size + theme.legend_marker_text_gap) as Precision,
                y: (item_y + row_h * 0.75) as Precision,
                font_size: font_size as Precision,
                font_family: font_family.clone(),
                color: theme.legend_label_color,
                text_anchor: "start".to_string(),
                font_weight: "normal".to_string(),
                opacity: 1.0,
            };
            backend.draw_text(text_config);

            item_y += row_h + theme.legend_item_v_gap;
        }

        GuideSize {
            width: total_w + current_col_w,
            height: if total_w > 0.0 { max_h } else { item_y - y },
        }
    }

    /// Maps data values into visual properties using the GlobalAesthetics context.
    #[allow(clippy::type_complexity)] // A type alias isn't needed for a single usage.
    fn resolve_mappings(
        spec: &GuideSpec,
        ctx: &PanelContext,
    ) -> (
        Vec<String>,
        Vec<SingleColor>,
        Option<Vec<PointShape>>,
        Option<Vec<f64>>,
    ) {
        let (labels, values_f64): (Vec<String>, Vec<f64>) = match &spec.domain {
            ScaleDomain::Discrete(values) => (values.clone(), Vec::new()),
            _ => {
                let ticks = spec.get_sampling_ticks();
                let l = ticks.iter().map(|t| t.label.clone()).collect();
                let v = ticks.iter().map(|t| t.value).collect();
                (l, v)
            }
        };

        let mut colors = Vec::new();
        let mut shapes = Vec::new();
        let mut sizes = Vec::new();

        // Check availability of specific mappers
        let has_color = spec.mappings.iter().any(|m| {
            m.scale_impl.mapper().is_some_and(|v| {
                matches!(
                    v,
                    VisualMapper::DiscreteColor { .. } | VisualMapper::ContinuousColor { .. }
                )
            })
        });
        let has_shape = spec.mappings.iter().any(|m| {
            m.scale_impl
                .mapper()
                .is_some_and(|v| matches!(v, VisualMapper::Shape { .. }))
        });
        let has_size = spec.mappings.iter().any(|m| {
            m.scale_impl
                .mapper()
                .is_some_and(|v| matches!(v, VisualMapper::Size { .. }))
        });

        for (i, label_str) in labels.iter().enumerate() {
            let val_f64 = values_f64.get(i).cloned();

            // Resolve Color
            if has_color {
                if let Some(ref mapping) = ctx.spec.aesthetics.color {
                    let norm = val_f64
                        .map(|v| mapping.scale_impl.normalize(v))
                        .unwrap_or_else(|| mapping.scale_impl.normalize_string(label_str));

                    let color = mapping
                        .scale_impl
                        .mapper()
                        .map(|m| m.map_to_color(norm, mapping.scale_impl.logical_max()))
                        .unwrap_or_else(|| "#333333".into());
                    colors.push(color);
                }
            } else {
                colors.push("#333333".into());
            }

            // Resolve Shape
            if has_shape {
                if let Some(ref mapping) = ctx.spec.aesthetics.shape {
                    let norm = val_f64
                        .map(|v| mapping.scale_impl.normalize(v))
                        .unwrap_or_else(|| mapping.scale_impl.normalize_string(label_str));

                    let shape = mapping
                        .scale_impl
                        .mapper()
                        .map(|m| m.map_to_shape(norm, mapping.scale_impl.logical_max()))
                        .unwrap_or(PointShape::Circle);
                    shapes.push(shape);
                }
            } else {
                shapes.push(PointShape::Circle);
            }

            // Resolve Size
            if has_size {
                if let Some(ref mapping) = ctx.spec.aesthetics.size {
                    let norm = val_f64
                        .map(|v| mapping.scale_impl.normalize(v))
                        .unwrap_or_else(|| mapping.scale_impl.normalize_string(label_str));

                    let size = mapping
                        .scale_impl
                        .mapper()
                        .map(|m| m.map_to_size(norm))
                        .unwrap_or(5.0);
                    sizes.push(size);
                }
            } else {
                sizes.push(5.0);
            }
        }

        (
            labels,
            colors,
            if has_shape { Some(shapes) } else { None },
            if has_size { Some(sizes) } else { None },
        )
    }

    fn draw_symbol(
        backend: &mut dyn RenderBackend,
        shape: &PointShape,
        cx: f64,
        cy: f64,
        r: f64,
        color: &SingleColor,
    ) {
        match shape {
            PointShape::Circle => {
                let circle_config = CircleConfig {
                    x: cx as Precision,
                    y: cy as Precision,
                    radius: r as Precision,
                    fill: *color,
                    stroke: SingleColor::new("none"),
                    stroke_width: 0.0,
                    opacity: 1.0,
                };
                backend.draw_circle(circle_config);
            }
            PointShape::Square => {
                let rect_config = RectConfig {
                    x: (cx - r) as Precision,
                    y: (cy - r) as Precision,
                    width: (r * 2.0) as Precision,
                    height: (r * 2.0) as Precision,
                    fill: *color,
                    stroke: SingleColor::new("none"),
                    stroke_width: 0.0,
                    opacity: 1.0,
                };
                backend.draw_rect(rect_config);
            }
            PointShape::Triangle => {
                let pts = vec![
                    (cx as Precision, (cy - r) as Precision),
                    ((cx - r) as Precision, (cy + r) as Precision),
                    ((cx + r) as Precision, (cy + r) as Precision),
                ];
                let polygon_config = PolygonConfig {
                    points: pts,
                    fill: *color,
                    stroke: SingleColor::new("none"),
                    stroke_width: 0.0,
                    fill_opacity: 1.0,
                    stroke_opacity: 1.0,
                };
                backend.draw_polygon(polygon_config);
            }
            PointShape::Diamond => {
                let pts = vec![
                    (cx as Precision, (cy - r) as Precision),
                    ((cx + r) as Precision, cy as Precision),
                    (cx as Precision, (cy + r) as Precision),
                    ((cx - r) as Precision, cy as Precision),
                ];
                let polygon_config = PolygonConfig {
                    points: pts,
                    fill: *color,
                    stroke: SingleColor::new("none"),
                    stroke_width: 0.0,
                    fill_opacity: 1.0,
                    stroke_opacity: 1.0,
                };
                backend.draw_polygon(polygon_config);
            }
            _ => {
                let circle_config = CircleConfig {
                    x: cx as Precision,
                    y: cy as Precision,
                    radius: r as Precision,
                    fill: *color,
                    stroke: SingleColor::new("none"),
                    stroke_width: 0.0,
                    opacity: 1.0,
                };
                backend.draw_circle(circle_config);
            }
        }
    }

    /// Calculates the initial (x, y) anchor for the legend block based on the panel position.
    fn calculate_initial_anchor(ctx: &PanelContext, theme: &Theme, _: bool) -> (f64, f64) {
        let mut x = ctx.panel.x;
        let mut y = ctx.panel.y;
        let margin = theme.legend_margin;
        let axis_buffer = theme.axis_reserve_buffer;

        match theme.legend_position {
            LegendPosition::Right => x = ctx.panel.x + ctx.panel.width + margin,
            LegendPosition::Left => x = (ctx.panel.x - margin - axis_buffer).max(10.0),
            LegendPosition::Top => y = (ctx.panel.y - margin - (axis_buffer * 0.8)).max(10.0),
            LegendPosition::Bottom => y = ctx.panel.y + ctx.panel.height + margin,
            _ => {}
        }
        (x, y)
    }
}
