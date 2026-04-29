use crate::Precision;
use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{
    CircleConfig, MarkRenderer, PointElementConfig, PolygonConfig, RectConfig, RenderBackend,
};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::point::{MarkPoint, PointLayout};
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (High-Performance Parallel Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkPoint> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        let row_count = df_source.height();
        if row_count == 0 {
            return Ok(());
        }

        // --- STEP 1: SPECIFICATION & ENCODINGS ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y missing".into()))?;
        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkPoint config missing".into()))?;

        // --- STEP 2: SCALES & NORMALIZATION ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();
        let is_flipped = context.coord.is_flipped();

        let unit_step_norm = (x_scale.normalize(1.0) - x_scale.normalize(0.0)).abs();

        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, df_source.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, df_source.column(&y_enc.field)?);

        let sub_idx_col = df_source.column(&format!("{}_sub_idx", TEMP_SUFFIX)).ok();
        let groups_count_col = df_source
            .column(&format!("{}_groups_count", TEMP_SUFFIX))
            .ok();

        let color_norms = context.spec.aesthetics.color.as_ref().map(|m| {
            m.scale_impl
                .scale_type()
                .normalize_column(m.scale_impl.as_ref(), df_source.column(&m.field).unwrap())
        });
        let size_norms = context.spec.aesthetics.size.as_ref().map(|m| {
            m.scale_impl
                .scale_type()
                .normalize_column(m.scale_impl.as_ref(), df_source.column(&m.field).unwrap())
        });
        let shape_norms = context.spec.aesthetics.shape.as_ref().map(|m| {
            m.scale_impl
                .scale_type()
                .normalize_column(m.scale_impl.as_ref(), df_source.column(&m.field).unwrap())
        });

        // --- STEP 3: LAYOUT EXECUTION ---
        let render_configs: Vec<PointElementConfig> = match mark_config.layout {
            PointLayout::Beeswarm => {
                // BEESWARM: Stateful collision resolution
                self.resolve_beeswarm_layout(
                    row_count,
                    &x_norms,
                    &y_norms,
                    &color_norms,
                    &size_norms,
                    &shape_norms,
                    sub_idx_col,
                    groups_count_col,
                    unit_step_norm,
                    context,
                    mark_config,
                )
            }
            _ => {
                // STANDARD / JITTER: Parallel processing
                (0..row_count)
                    .maybe_into_par_iter()
                    .filter_map(|i| {
                        let x_n = x_norms[i]?;
                        let y_n = y_norms[i]?;

                        let mut x_final_n = x_n;
                        let mut lane_width_norm = 0.0;

                        // Apply BoxPlot-style Dodge Logic to calculate categorical center
                        if let (Some(sub_col), Some(cnt_col)) = (sub_idx_col, groups_count_col) {
                            let total_groups = cnt_col.get_f64(i).unwrap_or(1.0);
                            let sub_idx = sub_col.get_f64(i).unwrap_or(0.0);

                            let box_width_data = mark_config.width.min(
                                mark_config.span
                                    / (total_groups + (total_groups - 1.0) * mark_config.spacing),
                            );
                            let box_width_norm = box_width_data * unit_step_norm;
                            let spacing_norm = box_width_norm * mark_config.spacing;

                            x_final_n += (sub_idx - (total_groups - 1.0) / 2.0)
                                * (box_width_norm + spacing_norm);
                            lane_width_norm = box_width_norm;
                        }

                        // Project logic coordinates to screen pixels
                        let (mut px, mut py) =
                            context.coord.transform(x_final_n, y_n, &context.panel);

                        // Pixel-based Jitter: Offset applied to categorical dimension
                        if matches!(mark_config.layout, PointLayout::Jitter) {
                            let seed = (i as u64).wrapping_mul(1103515245).wrapping_add(12345);
                            let noise = ((seed & 0x7FFFFFFF) as f64 / 2147483647.0) - 0.5;

                            // Adjust horizontal (px) or vertical (py) based on orientation
                            if is_flipped {
                                let lane_px_limit = lane_width_norm * context.panel.height;
                                py += noise * lane_px_limit;
                            } else {
                                let lane_px_limit = lane_width_norm * context.panel.width;
                                px += noise * lane_px_limit;
                            }
                        }

                        Some(self.build_element_config(
                            i,
                            px,
                            py,
                            &color_norms,
                            &size_norms,
                            &shape_norms,
                            context,
                            mark_config,
                        ))
                    })
                    .collect()
            }
        };

        // --- STEP 4: EMIT ---
        for config in render_configs {
            self.emit_draw_call(backend, config);
        }

        Ok(())
    }
}

impl Chart<MarkPoint> {
    fn resolve_beeswarm_layout(
        &self,
        row_count: usize,
        x_norms: &[Option<f64>],
        y_norms: &[Option<f64>],
        color_norms: &Option<Vec<Option<f64>>>,
        size_norms: &Option<Vec<Option<f64>>>,
        shape_norms: &Option<Vec<Option<f64>>>,
        sub_idx_col: Option<&crate::core::data::ColumnVector>,
        groups_count_col: Option<&crate::core::data::ColumnVector>,
        unit_step_norm: f64,
        context: &PanelContext,
        mark_config: &MarkPoint,
    ) -> Vec<PointElementConfig> {
        let mut configs = Vec::with_capacity(row_count);
        let mut occupancy: std::collections::HashMap<(usize, usize), Vec<(f64, f64, f64)>> =
            std::collections::HashMap::new();

        let is_flipped = context.coord.is_flipped();

        for i in 0..row_count {
            let x_n = match x_norms[i] {
                Some(v) => v,
                None => continue,
            };
            let y_n = match y_norms[i] {
                Some(v) => v,
                None => continue,
            };

            let mut x_final_n = x_n;
            let mut lane_id = 0;

            // Step 1: Categorical axis screen width calculation
            let mut lane_px_width = if is_flipped {
                unit_step_norm * mark_config.span * context.panel.height
            } else {
                unit_step_norm * mark_config.span * context.panel.width
            };

            if let (Some(sub_col), Some(cnt_col)) = (sub_idx_col, groups_count_col) {
                let total_groups = cnt_col.get_f64(i).unwrap_or(1.0);
                let sub_idx = sub_col.get_f64(i).unwrap_or(0.0);
                lane_id = sub_idx as usize;

                let box_width_data = mark_config.width.min(
                    mark_config.span / (total_groups + (total_groups - 1.0) * mark_config.spacing),
                );
                let box_width_norm = box_width_data * unit_step_norm;
                let spacing_norm = box_width_norm * mark_config.spacing;

                x_final_n +=
                    (sub_idx - (total_groups - 1.0) / 2.0) * (box_width_norm + spacing_norm);

                lane_px_width = if is_flipped {
                    box_width_norm * context.panel.height
                } else {
                    box_width_norm * context.panel.width
                };
            }

            // Get the base projection (the center point of the swarm lane)
            let (base_px, base_py) = context.coord.transform(x_final_n, y_n, &context.panel);
            let size = self.resolve_size_from_value(
                size_norms.as_ref().and_then(|n| n[i]),
                context,
                mark_config.size,
            );

            // Step 2: Symmetric Collision Resolution
            let cat_key = ((x_n * 1000.0) as usize, lane_id);
            let siblings = occupancy.entry(cat_key).or_insert_with(Vec::new);

            let mut best_displacement = 0.0;
            let max_shift = lane_px_width * 0.5;
            let mut found = false;
            let step_px = 1.0;

            let max_attempts = (max_shift / step_px) as i32 + 1;

            for offset_step in 0..max_attempts {
                for sign in [1.0, -1.0] {
                    if offset_step == 0 && sign == -1.0 {
                        continue;
                    }

                    let displacement = offset_step as f64 * step_px * sign;

                    // Determine test coordinates by applying displacement to the categorical axis
                    let (test_x, test_y) = if is_flipped {
                        (base_px, base_py + displacement)
                    } else {
                        (base_px + displacement, base_py)
                    };

                    let mut collision = false;
                    for &(ox, oy, or) in siblings.iter() {
                        let dx = test_x - ox;
                        let dy = test_y - oy;
                        let dist_sq = dx * dx + dy * dy;
                        let min_d = (size + or) * 1.02; // 2% visual buffer

                        if dist_sq < min_d * min_d {
                            collision = true;
                            break;
                        }
                    }

                    if !collision {
                        best_displacement = displacement;
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }

            // Step 3: Boundary Clamping (Force overlap at high density)
            if best_displacement.abs() > max_shift {
                best_displacement = best_displacement.signum() * max_shift;
            }

            // Apply final displacement to the categorical axis
            let (final_px, final_py) = if is_flipped {
                (base_px, base_py + best_displacement)
            } else {
                (base_px + best_displacement, base_py)
            };

            siblings.push((final_px, final_py, size));
            configs.push(self.build_element_config(
                i,
                final_px,
                final_py,
                &color_norms,
                &size_norms,
                &shape_norms,
                context,
                mark_config,
            ));
        }
        configs
    }

    /// Helper to build the visual configuration for a single point element.
    fn build_element_config(
        &self,
        i: usize,
        x: f64,
        y: f64,
        color_norms: &Option<Vec<Option<f64>>>,
        size_norms: &Option<Vec<Option<f64>>>,
        shape_norms: &Option<Vec<Option<f64>>>,
        context: &PanelContext,
        mark_config: &MarkPoint,
    ) -> PointElementConfig {
        PointElementConfig {
            x,
            y,
            fill: self.resolve_color_from_value(
                color_norms.as_ref().and_then(|n| n[i]),
                context,
                &mark_config.color,
            ),
            size: self.resolve_size_from_value(
                size_norms.as_ref().and_then(|n| n[i]),
                context,
                mark_config.size,
            ),
            shape: self.resolve_shape_from_value(
                shape_norms.as_ref().and_then(|n| n[i]),
                context,
                mark_config.shape,
            ),
            stroke: mark_config.stroke,
            stroke_width: mark_config.stroke_width,
            opacity: mark_config.opacity,
        }
    }
}

// ============================================================================
// HELPER METHODS & GEOMETRY DISPATCH
// ============================================================================

impl Chart<MarkPoint> {
    /// Maps a normalized value to a color using the registered scale mapper.
    fn resolve_color_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> SingleColor {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.color) {
            let s_trait = mapping.scale_impl.as_ref();
            s_trait
                .mapper()
                .as_ref()
                .map(|m| m.map_to_color(v, s_trait.logical_max()))
                .unwrap_or(*fallback)
        } else {
            *fallback
        }
    }

    /// Maps a normalized value to a point size.
    fn resolve_size_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: f64,
    ) -> f64 {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.size) {
            mapping
                .scale_impl
                .mapper()
                .as_ref()
                .map(|m| m.map_to_size(v))
                .unwrap_or(fallback)
        } else {
            fallback
        }
    }

    /// Maps a normalized value to a specific PointShape.
    fn resolve_shape_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: PointShape,
    ) -> PointShape {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.shape) {
            let s_trait = mapping.scale_impl.as_ref();
            mapping
                .scale_impl
                .mapper()
                .as_ref()
                .map(|m| m.map_to_shape(v, s_trait.logical_max()))
                .unwrap_or(fallback)
        } else {
            fallback
        }
    }

    /// Dispatches the appropriate backend draw call for the given PointShape.
    fn emit_draw_call(&self, backend: &mut dyn RenderBackend, config: PointElementConfig) {
        let PointElementConfig {
            x,
            y,
            shape,
            size,
            fill,
            stroke,
            stroke_width,
            opacity,
        } = config;

        match shape {
            PointShape::Circle => {
                backend.draw_circle(CircleConfig {
                    x: x as Precision,
                    y: y as Precision,
                    radius: size as Precision,
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    opacity: opacity as Precision,
                });
            }
            PointShape::Square => {
                let side = size * 2.0;
                backend.draw_rect(RectConfig {
                    x: (x - size) as Precision,
                    y: (y - size) as Precision,
                    width: side as Precision,
                    height: side as Precision,
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    opacity: opacity as Precision,
                });
            }
            _ => {
                let (sides, rotation, scale_adj) = match shape {
                    PointShape::Diamond => (4, 0.0, 1.2),
                    PointShape::Triangle => (3, -std::f64::consts::FRAC_PI_2, 1.1),
                    PointShape::Pentagon => (5, -std::f64::consts::FRAC_PI_2, 1.0),
                    PointShape::Hexagon => (6, 0.0, 1.0),
                    PointShape::Octagon => (8, std::f64::consts::FRAC_PI_8, 1.0),
                    _ => (0, 0.0, 0.0),
                };

                let points = if shape == PointShape::Star {
                    self.calculate_star(x, y, size * 1.2, size * 0.5, 5)
                } else {
                    self.calculate_polygon(x, y, size * scale_adj, sides, rotation)
                };

                backend.draw_polygon(PolygonConfig {
                    points: points
                        .iter()
                        .map(|p| (p.0 as Precision, p.1 as Precision))
                        .collect(),
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    fill_opacity: opacity as Precision,
                    stroke_opacity: 1.0,
                });
            }
        }
    }

    fn calculate_polygon(
        &self,
        cx: f64,
        cy: f64,
        r: f64,
        sides: usize,
        rot: f64,
    ) -> Vec<(f64, f64)> {
        (0..sides)
            .map(|i| {
                let angle = rot + 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
                (cx + r * angle.cos(), cy + r * angle.sin())
            })
            .collect()
    }

    fn calculate_star(
        &self,
        cx: f64,
        cy: f64,
        out_r: f64,
        in_r: f64,
        pts: usize,
    ) -> Vec<(f64, f64)> {
        (0..(pts * 2))
            .map(|i| {
                let angle =
                    -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * (i as f64) / (pts as f64);
                let r = if i % 2 == 0 { out_r } else { in_r };
                (cx + r * angle.cos(), cy + r * angle.sin())
            })
            .collect()
    }
}
