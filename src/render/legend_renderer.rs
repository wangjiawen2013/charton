use crate::Precision;
use crate::coordinate::Rect;
use crate::core::context::PanelContext;
use crate::core::flow::Direction;
use crate::core::guide::{ColorBarGeometry, GuideKind, GuideSpec, LegendPosition};
use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PolygonConfig, RectConfig, RenderBackend,
    TextConfig,
};
use crate::core::layout::{GuideBlockLayout, LegendLayoutPlan};
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
    /// `placement` is the strip the layout set aside for the legend; it decides
    /// where the strip starts. `band` is the open area on that side of the
    /// panel, and it is what the drawing is clipped to. The reserved strip is
    /// based on an estimated text width, so an estimate that is a little short
    /// leaves the label sticking out of the strip. Clipping to the wider band
    /// keeps such a label whole, while the panel stays out of the band, so a
    /// legend can still never cover the data.
    pub fn render_legend<B: RenderBackend>(
        backend: &mut B,
        specs: &[GuideSpec],
        plan: &LegendLayoutPlan,
        theme: &Theme,
        ctx: &PanelContext,
        placement: &Rect,
        band: &Rect,
    ) {
        // Resolve the legend position from the theme.
        let position = plan.position;

        if specs.is_empty() || matches!(position, LegendPosition::None) {
            return;
        }

        let font_size = theme.legend_label_size;
        let font_family = &theme.legend_label_family;

        let direction = Direction::for_legend(position);
        let is_horizontal = matches!(direction, Direction::Horizontal);

        // The strip hugs the panel on its cross axis and starts at the outer
        // edge of the reserved strip. Because `placement` is the strip the
        // layout set aside, the drawing and the reserved space always line up.
        let (origin_x, origin_y) = match position {
            LegendPosition::Top | LegendPosition::Bottom => (ctx.panel.x, placement.y),
            LegendPosition::Left | LegendPosition::Right => (placement.x, ctx.panel.y),
            LegendPosition::None => (ctx.panel.x, ctx.panel.y),
        };
        Self::warn_if_clipped(plan, ctx, origin_x, origin_y, band);

        // Only ever paint inside the open area on this side of the panel, so a
        // legend that is far too big is cut off instead of spilling onto the
        // title or the plot.
        backend.begin_clip_scope(band);

        // Every block and every entry already knows where it goes; this loop only
        // draws. It must not re-derive any layout, or the drawing would start
        // disagreeing with the space the layout reserved.
        for block in &plan.blocks {
            let Some(spec) = specs.get(block.index) else {
                continue;
            };
            let block_x = origin_x + block.offset_x;
            let block_y = origin_y + block.offset_y;

            // 1. Draw the block title.
            backend.draw_text(TextConfig {
                text: spec.title.clone(),
                x: block_x as Precision,
                y: (block_y + (font_size / 2.0)) as Precision,
                font_size: (font_size * 1.1) as Precision,
                font_family: font_family.clone(),
                color: theme.legend_title_color,
                text_anchor: "start".to_string(),
                dominant_baseline: "central".into(),
                font_weight: "bold".to_string(),
                opacity: 1.0,
                angle: 0.0,
            });

            // 2. Draw the content: a gradient bar, or the discrete entries.
            // Entry offsets are relative to the block origin and already sit
            // below the title, so no extra offset is needed here.
            match spec.kind {
                GuideKind::ColorBar => {
                    // Colour bars are guaranteed to carry their geometry.
                    if let Some(geometry) = block.colorbar {
                        Self::draw_colorbar(
                            backend,
                            spec,
                            ctx,
                            block_x,
                            block_y + (font_size * 1.1) + theme.legend_title_gap,
                            theme,
                            is_horizontal,
                            geometry,
                        );
                    }
                }
                GuideKind::Legend => {
                    let (labels, colors, shapes, sizes) = Self::resolve_mappings(spec, ctx);
                    Self::draw_entries(
                        backend,
                        &labels,
                        &colors,
                        shapes.as_deref(),
                        sizes.as_deref(),
                        block,
                        block_x,
                        block_y,
                        font_size,
                        theme,
                    );
                }
            };
        }

        backend.end_clip_scope();
    }

    /// Whether `inner` lies completely inside `outer`.
    ///
    /// Split out from the warning below so the condition can be unit tested
    /// without having to capture stderr.
    fn fits_inside(outer: &Rect, inner: &Rect) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && inner.x + inner.width <= outer.x + outer.width
            && inner.y + inner.height <= outer.y + outer.height
    }

    /// Reports a legend strip that does not fit the band it is drawn in.
    ///
    /// The layout shrinks the plot panel to make room for the legend, but only
    /// down to a floor (`theme.min_panel_size`, guarded by
    /// `theme.panel_defense_ratio`), so a legend that is simply too large for the
    /// canvas can end up bigger than the open space around the panel. The clip
    /// below then cuts it off at the edge of that space: the leading entries are
    /// drawn and the rest silently disappears. That is the right trade-off -- a
    /// clean plot beats a legend painted over the data -- but it should never
    /// happen unnoticed, hence this line. It fires once per rendered sheet, and
    /// never on the GPU path, which does not draw legends at all.
    fn warn_if_clipped(
        plan: &LegendLayoutPlan,
        ctx: &PanelContext,
        origin_x: f64,
        origin_y: f64,
        band: &Rect,
    ) {
        let strip = Rect::new(origin_x, origin_y, plan.width, plan.height);
        if Self::fits_inside(band, &strip) {
            return;
        }

        eprintln!(
            "Legend: Clipped a {:.0}x{:.0} strip to the {:.0}x{:.0} band on the {:?} side \
             (panel kept at {:.0}x{:.0}).",
            strip.width,
            strip.height,
            band.width,
            band.height,
            plan.position,
            ctx.panel.width,
            ctx.panel.height
        );
    }

    /// Renders a continuous color gradient bar (ColorBar).
    ///
    /// The bar's size, its ticks and its labels all come from the layout plan, so
    /// what is drawn here is exactly the box that was reserved for it.
    #[allow(clippy::too_many_arguments)]
    fn draw_colorbar<B: RenderBackend>(
        backend: &mut B,
        spec: &GuideSpec,
        ctx: &PanelContext,
        x: f64,
        y: f64,
        theme: &Theme,
        is_horizontal: bool,
        geometry: ColorBarGeometry,
    ) {
        // The gradient runs along the bar, so on a vertical bar the measured
        // length becomes the drawn height.
        let (bar_w, bar_h) = if is_horizontal {
            (geometry.length, geometry.thickness)
        } else {
            (geometry.thickness, geometry.length)
        };
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
            is_vertical: !is_horizontal,
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

        if let Some(mapping) = spec.mappings.first() {
            for tick in spec.colorbar_ticks() {
                let norm = mapping.scale_impl.normalize(tick.value);
                if is_horizontal {
                    let tick_x = x + bar_w * norm;
                    backend.draw_line(LineConfig {
                        x1: tick_x as Precision,
                        y1: y as Precision,
                        x2: tick_x as Precision,
                        y2: (y + 3.0) as Precision,
                        color: "#FFFFFF".into(),
                        width: 1.0,
                        opacity: 1.0,
                        dash: vec![],
                    });
                    backend.draw_text(TextConfig {
                        text: tick.label.clone(),
                        x: tick_x as Precision,
                        y: (y + bar_h + theme.tick_label_padding) as Precision,
                        font_size: font_size as Precision,
                        font_family: font_family.clone(),
                        color: theme.legend_label_color,
                        text_anchor: "middle".to_string(),
                        dominant_baseline: "hanging".into(),
                        font_weight: "normal".to_string(),
                        opacity: 1.0,
                        angle: 0.0,
                    });
                } else {
                    let tick_y = y + (bar_h * (1.0 - norm));
                    backend.draw_line(LineConfig {
                        x1: x as Precision,
                        y1: tick_y as Precision,
                        x2: (x + 3.0) as Precision,
                        y2: tick_y as Precision,
                        color: "#FFFFFF".into(),
                        width: 1.0,
                        opacity: 1.0,
                        dash: vec![],
                    });
                    backend.draw_line(LineConfig {
                        x1: (x + bar_w - 3.0) as Precision,
                        y1: tick_y as Precision,
                        x2: (x + bar_w) as Precision,
                        y2: tick_y as Precision,
                        color: "#FFFFFF".into(),
                        width: 1.0,
                        opacity: 1.0,
                        dash: vec![],
                    });
                    backend.draw_text(TextConfig {
                        text: tick.label.clone(),
                        x: (x + bar_w + theme.legend_marker_text_gap) as Precision,
                        y: tick_y as Precision,
                        font_size: font_size as Precision,
                        font_family: font_family.clone(),
                        color: theme.legend_label_color,
                        text_anchor: "start".to_string(),
                        dominant_baseline: "central".into(),
                        font_weight: "normal".to_string(),
                        opacity: 1.0,
                        angle: 0.0,
                    });
                }
            }
        }
    }

    /// Draws the discrete entries of one legend block.
    ///
    /// Every position comes from the layout plan, which resolved symbol and label
    /// placement while measuring. This function therefore never decides *where*
    /// an entry goes -- it only centres each symbol in its pre-computed cell.
    #[allow(clippy::too_many_arguments)]
    fn draw_entries(
        backend: &mut dyn RenderBackend,
        labels: &[String],
        colors: &[SingleColor],
        shapes: Option<&[PointShape]>,
        sizes: Option<&[f64]>,
        block: &GuideBlockLayout,
        block_x: f64,
        block_y: f64,
        font_size: f64,
        theme: &Theme,
    ) {
        let font_family = &theme.legend_label_family;
        let cell = block.metrics.cell;
        let row = block.metrics.row;

        for (index, entry) in block.entries.iter().enumerate() {
            let Some(label) = labels.get(index) else {
                break;
            };
            let radius = sizes
                .and_then(|values| values.get(index))
                .cloned()
                .unwrap_or(5.0);
            let shape = shapes
                .and_then(|values| values.get(index))
                .unwrap_or(&PointShape::Circle);

            // The symbol and its label share the vertical centre of the row.
            let centre_y = block_y + entry.y + row / 2.0;

            Self::draw_symbol(
                backend,
                shape,
                block_x + entry.x + (cell / 2.0),
                centre_y,
                radius,
                colors.get(index).unwrap_or(&"#333333".into()),
            );

            backend.draw_text(TextConfig {
                text: label.clone(),
                x: (block_x + entry.x + cell + theme.legend_marker_text_gap) as Precision,
                y: centre_y as Precision,
                font_size: font_size as Precision,
                font_family: font_family.clone(),
                color: theme.legend_label_color,
                text_anchor: "start".to_string(),
                dominant_baseline: "central".into(),
                font_weight: "normal".to_string(),
                opacity: 1.0,
                angle: 0.0,
            });
        }
    }

    /// Maps data values into visual properties using the GlobalAesthetics context.
    #[allow(clippy::type_complexity)]
    fn resolve_mappings(
        spec: &GuideSpec,
        ctx: &PanelContext,
    ) -> (
        Vec<String>,
        Vec<SingleColor>,
        Option<Vec<PointShape>>,
        Option<Vec<f64>>,
    ) {
        // `labels` are shown to the user and may have been formatted.
        // `lookup_labels` are the original category names used to look up
        // colours and shapes. Formatting must not affect that lookup.
        let (labels, lookup_labels, values_f64): (Vec<String>, Vec<String>, Vec<f64>) =
            match &spec.domain {
                ScaleDomain::Discrete(values) => {
                    (spec.get_sampling_labels(), values.clone(), Vec::new())
                }
                _ => {
                    let ticks = spec.get_sampling_ticks();
                    let l = ticks.iter().map(|t| t.label.clone()).collect();
                    let v = ticks.iter().map(|t| t.value).collect();
                    (l, Vec::new(), v)
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
            let lookup = lookup_labels.get(i).unwrap_or(label_str);

            // Resolve Color
            if has_color {
                if let Some(ref mapping) = ctx.spec.aesthetics.color {
                    let norm = val_f64
                        .map(|v| mapping.scale_impl.normalize(v))
                        .unwrap_or_else(|| mapping.scale_impl.normalize_string(lookup));

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
                        .unwrap_or_else(|| mapping.scale_impl.normalize_string(lookup));

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
                        .unwrap_or_else(|| mapping.scale_impl.normalize_string(lookup));

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

    /// Renders a single geometric symbol based on the PointShape variant.
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
                backend.draw_circle(CircleConfig {
                    x: cx as Precision,
                    y: cy as Precision,
                    radius: r as Precision,
                    fill: *color,
                    stroke: SingleColor::new("none"),
                    stroke_width: 0.0,
                    opacity: 1.0,
                });
            }
            PointShape::Square => {
                // Scale factor = sqrt(pi / 4) ≈ 0.886 to equalize area with a circle of radius r
                let adj_r = r * 0.88623;
                backend.draw_rect(RectConfig {
                    x: (cx - adj_r) as Precision,
                    y: (cy - adj_r) as Precision,
                    width: (adj_r * 2.0) as Precision,
                    height: (adj_r * 2.0) as Precision,
                    fill: *color,
                    stroke: SingleColor::new("none"),
                    stroke_width: 0.0,
                    opacity: 1.0,
                });
            }
            // For all other geometric shapes, we calculate vertices and use draw_polygon
            _ => {
                // Apply the same area-equalizing scale adjustments as in the data marks
                let scale_adj = match shape {
                    PointShape::Diamond => 1.253,
                    PointShape::Triangle => 1.555,
                    PointShape::Pentagon => 1.150,
                    PointShape::Hexagon => 1.099,
                    PointShape::Octagon => 1.054,
                    _ => 1.0,
                };

                let points = match shape {
                    PointShape::Triangle => Self::gen_regular_poly(cx, cy, r * scale_adj, 3, -90.0),
                    PointShape::Diamond => Self::gen_regular_poly(cx, cy, r * scale_adj, 4, 0.0),
                    PointShape::Pentagon => Self::gen_regular_poly(cx, cy, r * scale_adj, 5, -90.0),
                    PointShape::Hexagon => Self::gen_regular_poly(cx, cy, r * scale_adj, 6, 0.0),
                    PointShape::Octagon => Self::gen_regular_poly(cx, cy, r * scale_adj, 8, 22.5),
                    // For Star, roughly 1.6 outer and 0.6 inner matches the circle area
                    PointShape::Star => Self::gen_star(cx, cy, r * 1.6, r * 0.6, 5),
                    _ => Vec::new(),
                };

                if !points.is_empty() {
                    backend.draw_polygon(PolygonConfig {
                        points,
                        fill: *color,
                        stroke: SingleColor::new("none"),
                        stroke_width: 0.0,
                        opacity: 1.0,
                    });
                } else {
                    // Final fallback to Circle if shape is undefined
                    backend.draw_circle(CircleConfig {
                        x: cx as Precision,
                        y: cy as Precision,
                        radius: r as Precision,
                        fill: *color,
                        stroke: SingleColor::new("none"),
                        stroke_width: 0.0,
                        opacity: 1.0,
                    });
                }
            }
        }
    }

    /// Generates vertices for a regular polygon inscribed in a circle of radius r.
    /// rotation_deg: Offset angle in degrees (e.g., -90 to point the first vertex upward).
    fn gen_regular_poly(
        cx: f64,
        cy: f64,
        r: f64,
        sides: usize,
        rotation_deg: f64,
    ) -> Vec<(Precision, Precision)> {
        let mut pts = Vec::with_capacity(sides);
        let start_angle = rotation_deg.to_radians();
        for i in 0..sides {
            let angle = start_angle + (i as f64 * 2.0 * std::f64::consts::PI / sides as f64);
            pts.push((
                (cx + r * angle.cos()) as Precision,
                (cy + r * angle.sin()) as Precision,
            ));
        }
        pts
    }

    /// Generates vertices for a star shape.
    /// inner_r: The radius of the inner vertices (points of the star).
    /// points: Number of star points (e.g., 5 for a standard pentagram).
    fn gen_star(
        cx: f64,
        cy: f64,
        outer_r: f64,
        inner_r: f64,
        points: usize,
    ) -> Vec<(Precision, Precision)> {
        let mut pts = Vec::with_capacity(points * 2);
        let start_angle = -std::f64::consts::PI / 2.0; // Point upwards
        for i in 0..(points * 2) {
            let curr_r = if i % 2 == 0 { outer_r } else { inner_r };
            let angle = start_angle + (i as f64 * std::f64::consts::PI / points as f64);
            pts.push((
                (cx + curr_r * angle.cos()) as Precision,
                (cy + curr_r * angle.sin()) as Precision,
            ));
        }
        pts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect::new(x, y, w, h)
    }

    #[test]
    fn fits_inside_accepts_a_strip_touching_the_band_edges() {
        let band = rect(200.0, 0.0, 520.0, 480.0);
        // Exactly filling the band still counts as a fit: nothing is cut off.
        assert!(LegendRenderer::fits_inside(
            &band,
            &rect(200.0, 0.0, 520.0, 480.0)
        ));
        assert!(LegendRenderer::fits_inside(
            &band,
            &rect(210.0, 10.0, 100.0, 100.0)
        ));
    }

    #[test]
    fn fits_inside_rejects_overflow_on_every_side() {
        let band = rect(200.0, 0.0, 520.0, 480.0);

        // Starts before the band (the clamped left/top case).
        assert!(!LegendRenderer::fits_inside(
            &band,
            &rect(190.0, 0.0, 100.0, 100.0)
        ));
        assert!(!LegendRenderer::fits_inside(
            &band,
            &rect(200.0, -10.0, 100.0, 100.0)
        ));
        // Runs past the band (the clipped right/bottom case).
        assert!(!LegendRenderer::fits_inside(
            &band,
            &rect(200.0, 0.0, 521.0, 100.0)
        ));
        assert!(!LegendRenderer::fits_inside(
            &band,
            &rect(200.0, 0.0, 100.0, 481.0)
        ));
    }
}
