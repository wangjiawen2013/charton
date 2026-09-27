use crate::Precision;
use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{
    CircleConfig, MarkRenderer, PointElementConfig, PolygonConfig, RectConfig, RenderBackend,
};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::point::{MarkPoint, PointLayout, QuasirandomMethod};
use crate::stats::kde::{BandwidthType, density_profile};
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Small extra gap kept between two markers in a swarm, expressed as a
/// multiplier on their combined radius. It stops markers from looking glued
/// together when they are almost touching.
const BEESWARM_COLLISION_BUFFER: f64 = 1.02;

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

        let color_norms = context.spec.aesthetics.color.as_ref().and_then(|m| {
            let s = m.scale_impl.as_ref();
            let col = df_source.column(&m.field).ok()?;
            Some(s.scale_type().normalize_column(s, col))
        });
        let size_norms = context.spec.aesthetics.size.as_ref().and_then(|m| {
            let s = m.scale_impl.as_ref();
            let col = df_source.column(&m.field).ok()?;
            Some(s.scale_type().normalize_column(s, col))
        });
        let shape_norms = context.spec.aesthetics.shape.as_ref().and_then(|m| {
            let s = m.scale_impl.as_ref();
            let col = df_source.column(&m.field).ok()?;
            Some(s.scale_type().normalize_column(s, col))
        });

        // --- STEP 3: LAYOUT EXECUTION ---
        // Note: We now return a tuple of (row_index, PointElementConfig) to retain
        // the mapping between the calculated geometry and its original row in the dataset.
        let render_configs: Vec<(usize, PointElementConfig)> = match mark_config.layout {
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
            PointLayout::Quasirandom => {
                // QUASIRANDOM: Density aware spread in data space
                self.resolve_quasirandom_layout(
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
                            let total_groups = cnt_col.get(i).to_f64().unwrap_or(1.0);
                            let sub_idx = sub_col.get(i).to_f64().unwrap_or(0.0);

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

                        // Return the original row index 'i' alongside the config
                        Some((
                            i,
                            self.build_element_config(
                                i,
                                px,
                                py,
                                &color_norms,
                                &size_norms,
                                &shape_norms,
                                context,
                                mark_config,
                            ),
                        ))
                    })
                    .collect()
            }
        };

        // --- STEP 4: GROUPING & EMISSION ---
        // Determine the field to group by for deterministic Z-indexing and WGPU batching.
        // We prioritize Color, then Shape. If neither is mapped, group_by(None) will
        // put all points in a single, massive continuous batch.
        let group_field = context
            .spec
            .aesthetics
            .color
            .as_ref()
            .map(|c| &c.field)
            .or_else(|| context.spec.aesthetics.shape.as_ref().map(|s| &s.field));

        // group_by guarantees "First Appearance" order.
        let grouped_indices = df_source.group_by(group_field.map(|s| s.as_str()));

        // Create a fast lookup table to map row indices to their computed rendering configs.
        // We use repeat_with to avoid requiring the Clone trait on PointElementConfig.
        let mut config_lookup: Vec<Option<PointElementConfig>> =
            std::iter::repeat_with(|| None).take(row_count).collect();

        for (i, config) in render_configs {
            config_lookup[i] = Some(config);
        }

        // Emit draw calls sequentially by group.
        // This ensures identical shapes/colors are drawn contiguously, massively reducing
        // pipeline state changes (Draw Calls) in the WGPU backend via interleaved batching.
        for (_group_key, row_indices) in grouped_indices.groups {
            for &idx in &row_indices {
                // take() moves the value out, leaving None, which is perfectly safe and fast.
                if let Some(config) = config_lookup[idx].take() {
                    self.emit_draw_call(backend, config);
                }
            }
        }

        Ok(())
    }
}

impl Chart<MarkPoint> {
    /// Arranges the points of a point mark into "beeswarm" clusters.
    ///
    /// A beeswarm keeps every marker at its value but shifts it along the
    /// categorical axis until it no longer covers another marker. A crowded
    /// strip of dots therefore becomes a compact cluster whose outline shows
    /// how the values are distributed.
    ///
    /// Points are arranged one lane at a time. A lane is the slot reserved for
    /// one color (or shape) group inside a category, so groups that share a
    /// category sit side by side and stay comparable, exactly like the dodged
    /// bars, boxes and error bars.
    ///
    /// Within a lane the placement is deterministic:
    ///
    /// 1. Points are handled from the lowest to the highest value.
    /// 2. A point starts at the lane center and is moved outward, alternating
    ///    sides, until it finds a spot that does not touch an already placed
    ///    marker.
    /// 3. If the lane is too crowded to hold the point, it is pressed against
    ///    the roomier lane edge. This keeps the few unavoidable overlaps at the
    ///    edge of the lane instead of piling points up on the center line.
    ///
    /// Geometry is computed in device pixels because the collision test must
    /// use the same size the markers are drawn with.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
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
    ) -> Vec<(usize, PointElementConfig)> {
        let mut configs = Vec::with_capacity(row_count);
        let mut occupancy: std::collections::HashMap<(usize, usize), Vec<(f64, f64, f64)>> =
            std::collections::HashMap::new();

        let is_flipped = context.coord.is_flipped();

        // Lane number of each point. Computing it up front lets the placement
        // order below group points by lane without re-reading the data.
        let lane_ids: Vec<usize> = (0..row_count)
            .map(|i| {
                sub_idx_col
                    .and_then(|col| col.get(i).to_f64())
                    .unwrap_or(0.0) as usize
            })
            .collect();

        // Decide the order in which points are placed: lane by lane, and inside
        // a lane from the lowest value upward. Neighbouring values then end up
        // as neighbours in the swarm, which is what gives it a regular shape.
        let mut order: Vec<usize> = (0..row_count)
            .filter(|&i| x_norms[i].is_some() && y_norms[i].is_some())
            .collect();
        order.sort_by(|&a, &b| {
            let group_a = ((x_norms[a].unwrap_or(0.0) * 1000.0) as usize, lane_ids[a]);
            let group_b = ((x_norms[b].unwrap_or(0.0) * 1000.0) as usize, lane_ids[b]);
            group_a.cmp(&group_b).then_with(|| {
                y_norms[a]
                    .unwrap_or(0.0)
                    .partial_cmp(&y_norms[b].unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        for &i in &order {
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
                let total_groups = cnt_col.get(i).to_f64().unwrap_or(1.0);
                let sub_idx = sub_col.get(i).to_f64().unwrap_or(0.0);
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

            // Step 2: Place the marker on the first free spot, starting at the
            // lane center and stepping outward to both sides.
            let cat_key = ((x_n * 1000.0) as usize, lane_id);
            let siblings = occupancy.entry(cat_key).or_default();

            let max_shift = lane_px_width * 0.5;
            let step_px = 1.0;
            let max_attempts = (max_shift / step_px) as i32 + 1;

            let mut free_displacement: Option<f64> = None;

            'search: for offset_step in 0..max_attempts {
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

                    if !Self::beeswarm_overlaps(siblings.as_slice(), test_x, test_y, size) {
                        free_displacement = Some(displacement);
                        break 'search;
                    }
                }
            }

            // Step 3: When a lane is full, press the point against the roomier
            // edge, so the overlap it cannot avoid lands at the fringe of the
            // lane rather than on the center line.
            let displacement = match free_displacement {
                Some(d) => d,
                None => {
                    let mut chosen = 0.0;
                    let mut least_overlap = f64::INFINITY;
                    for sign in [1.0, -1.0] {
                        let edge = sign * max_shift;
                        let (test_x, test_y) = if is_flipped {
                            (base_px, base_py + edge)
                        } else {
                            (base_px + edge, base_py)
                        };
                        let overlap =
                            Self::beeswarm_overlap(siblings.as_slice(), test_x, test_y, size);
                        if overlap < least_overlap {
                            least_overlap = overlap;
                            chosen = edge;
                        }
                    }
                    chosen
                }
            };

            // Apply the final displacement to the categorical axis
            let (final_px, final_py) = if is_flipped {
                (base_px, base_py + displacement)
            } else {
                (base_px + displacement, base_py)
            };

            siblings.push((final_px, final_py, size));

            configs.push((
                i,
                self.build_element_config(
                    i,
                    final_px,
                    final_py,
                    color_norms,
                    size_norms,
                    shape_norms,
                    context,
                    mark_config,
                ),
            ));
        }
        configs
    }

    /// Spreads the points of a point mark into a density aware cloud.
    ///
    /// Each point keeps its value but slides along the categorical axis. How
    /// far it may slide is decided by how crowded the data is around that
    /// value: a point inside a dense region is allowed to move to the full
    /// width of its lane, while a lone point stays near the center. The union
    /// of all points therefore traces the outline of the distribution instead
    /// of a straight column, which is the look usually called a violin cloud.
    ///
    /// Points are handled one lane at a time. A lane is the slot reserved for
    /// one color (or shape) group inside a category, so groups that share a
    /// category sit side by side and stay comparable, exactly like the dodged
    /// bars, boxes and error bars.
    ///
    /// The offset is computed in normalized coordinates, so the outline does
    /// not depend on the size of the output canvas.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn resolve_quasirandom_layout(
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
    ) -> Vec<(usize, PointElementConfig)> {
        // Group the points into lanes. Each lane gets its own density estimate
        // so that one group cannot flatten the shape of another.
        let mut lanes: std::collections::HashMap<(usize, usize), Vec<usize>> =
            std::collections::HashMap::new();

        for i in 0..row_count {
            if x_norms[i].is_none() || y_norms[i].is_none() {
                continue;
            }
            let lane_id = sub_idx_col
                .and_then(|col| col.get(i).to_f64())
                .unwrap_or(0.0) as usize;
            let category = (x_norms[i].unwrap_or(0.0) * 1000.0) as usize;
            lanes.entry((category, lane_id)).or_default().push(i);
        }

        let mut configs = Vec::with_capacity(row_count);

        for members in lanes.into_values() {
            // Density at every value in the lane, scaled so the busiest value
            // has a density of one.
            let values: Vec<f64> = members.iter().map(|&i| y_norms[i].unwrap_or(0.0)).collect();
            let density = density_profile(&values, &BandwidthType::Silverman);

            // Visit the lane from the lowest value upward. Neighbouring values
            // then receive opposite offsets, which keeps the cloud balanced
            // instead of pushing every point to the same side.
            let mut order: Vec<usize> = (0..members.len()).collect();
            order.sort_by(|&a, &b| {
                values[a]
                    .partial_cmp(&values[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Build the signed offset for every point in the lane. The sequence
            // is then centered and stretched so that a lane with only a few
            // points still spreads symmetrically instead of leaning to one
            // side. This is what keeps a lone point at the center.
            let mut sides: Vec<f64> = (0..members.len())
                .map(|rank| match mark_config.quasirandom_method {
                    QuasirandomMethod::Tukey => 2.0 * van_der_corput(rank as u64) - 1.0,
                    QuasirandomMethod::Pseudorandom => {
                        2.0 * pseudorandom_unit(members[rank] as u64) - 1.0
                    }
                })
                .collect();
            center_and_scale(&mut sides);

            for (rank, &slot) in order.iter().enumerate() {
                let i = members[slot];
                let x_n = x_norms[i].unwrap_or(0.0);
                let y_n = y_norms[i].unwrap_or(0.0);

                let (lane_center_n, lane_width_norm) = Self::point_lane(
                    i,
                    x_n,
                    sub_idx_col,
                    groups_count_col,
                    unit_step_norm,
                    mark_config,
                );

                let offset_n = 0.5 * lane_width_norm * density[slot] * sides[rank];
                let (px, py) =
                    context
                        .coord
                        .transform(lane_center_n + offset_n, y_n, &context.panel);

                configs.push((
                    i,
                    self.build_element_config(
                        i,
                        px,
                        py,
                        color_norms,
                        size_norms,
                        shape_norms,
                        context,
                        mark_config,
                    ),
                ));
            }
        }

        configs
    }

    /// Returns the center and the usable width of the lane that holds a point.
    ///
    /// A lane is the slot reserved for one color (or shape) group inside a
    /// category. When no grouping aesthetic is present the whole category is
    /// one lane. The returned width is the maximum horizontal space the point
    /// may use, expressed in normalized coordinates.
    fn point_lane(
        i: usize,
        x_n: f64,
        sub_idx_col: Option<&crate::core::data::ColumnVector>,
        groups_count_col: Option<&crate::core::data::ColumnVector>,
        unit_step_norm: f64,
        mark_config: &MarkPoint,
    ) -> (f64, f64) {
        match (sub_idx_col, groups_count_col) {
            (Some(sub_col), Some(cnt_col)) => {
                let total_groups = cnt_col.get(i).to_f64().unwrap_or(1.0);
                let sub_idx = sub_col.get(i).to_f64().unwrap_or(0.0);

                let box_width_data = mark_config.width.min(
                    mark_config.span / (total_groups + (total_groups - 1.0) * mark_config.spacing),
                );
                let box_width_norm = box_width_data * unit_step_norm;
                let spacing_norm = box_width_norm * mark_config.spacing;
                let center =
                    x_n + (sub_idx - (total_groups - 1.0) / 2.0) * (box_width_norm + spacing_norm);

                (center, box_width_norm)
            }
            _ => (x_n, mark_config.width * unit_step_norm),
        }
    }

    /// Returns `true` when a marker at `(x, y)` would cover a marker that has
    /// already been placed in the same swarm lane.
    fn beeswarm_overlaps(placed: &[(f64, f64, f64)], x: f64, y: f64, radius: f64) -> bool {
        placed.iter().any(|&(ox, oy, or)| {
            let dx = x - ox;
            let dy = y - oy;
            let min_distance = (radius + or) * BEESWARM_COLLISION_BUFFER;
            dx * dx + dy * dy < min_distance * min_distance
        })
    }

    /// Measures how deeply a marker at `(x, y)` overlaps the markers already
    /// placed in the same lane. Zero means a free spot; larger values mean a
    /// deeper overlap, which is used to choose the roomier lane edge.
    fn beeswarm_overlap(placed: &[(f64, f64, f64)], x: f64, y: f64, radius: f64) -> f64 {
        placed
            .iter()
            .map(|&(ox, oy, or)| {
                let dx = x - ox;
                let dy = y - oy;
                let distance = (dx * dx + dy * dy).sqrt();
                let min_distance = (radius + or) * BEESWARM_COLLISION_BUFFER;
                (min_distance - distance).max(0.0)
            })
            .sum()
    }

    /// Helper to build the visual configuration for a single point element.
    #[allow(clippy::too_many_arguments)]
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
                // Scale factor = sqrt(pi / 4) ≈ 0.886 to equalize square area with baseline circle
                let adj_size = size * 0.88623;
                backend.draw_rect(RectConfig {
                    x: (x - adj_size) as Precision,
                    y: (y - adj_size) as Precision,
                    width: (adj_size * 2.0) as Precision,
                    height: (adj_size * 2.0) as Precision,
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    opacity: opacity as Precision,
                });
            }
            _ => {
                // scale_adj is calculated to equalize the physical pixel area of each shape
                // against a baseline Circle of the same size (radius).
                let (sides, rotation, scale_adj) = match shape {
                    // Diamond (Square rotated 45 deg): Area = 2 * r^2. Circle Area = pi * r^2.
                    // Scale factor = sqrt(pi / 2) ≈ 1.253
                    PointShape::Diamond => (4, 0.0, 1.253),

                    // Equilateral Triangle: Area = (3 * sqrt(3) / 4) * r^2 ≈ 1.299 * r^2.
                    // Scale factor = sqrt(pi / 1.299) ≈ 1.555
                    PointShape::Triangle => (3, -std::f64::consts::FRAC_PI_2, 1.555),

                    // Regular Pentagon: Area ≈ 2.377 * r^2.
                    // Scale factor = sqrt(pi / 2.377) ≈ 1.150
                    PointShape::Pentagon => (5, -std::f64::consts::FRAC_PI_2, 1.150),

                    // Regular Hexagon: Area = (3 * sqrt(3) / 2) * r^2 ≈ 2.598 * r^2.
                    // Scale factor = sqrt(pi / 2.598) ≈ 1.099
                    PointShape::Hexagon => (6, 0.0, 1.099),

                    // Regular Octagon: Area = 2 * sqrt(2) * r^2 ≈ 2.828 * r^2.
                    // Scale factor = sqrt(pi / 2.828) ≈ 1.054
                    PointShape::Octagon => (8, std::f64::consts::FRAC_PI_8, 1.054),

                    _ => (0, 0.0, 0.0),
                };

                let points = if shape == PointShape::Star {
                    // For Star, we adjust the outer radius to roughly match circle area
                    // A 5-point star with inner_r = 0.382 * outer_r needs ~1.6 scale to match circle area
                    self.calculate_star(x, y, size * 1.6, size * 0.6, 5)
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
                    opacity: opacity as Precision,
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

/// Radical inverse of `index` in base two.
///
/// The sequence it produces (0, 0.5, 0.25, 0.75, 0.125, ...) covers the unit
/// interval evenly and never repeats a coarse pattern. A quasirandom layout
/// uses it to decide which side of the lane each point takes.
fn van_der_corput(mut index: u64) -> f64 {
    let mut result = 0.0;
    let mut fraction = 0.5;
    while index > 0 {
        if index & 1 == 1 {
            result += fraction;
        }
        index >>= 1;
        fraction *= 0.5;
    }
    result
}

/// Deterministic value in `[0, 1)` derived from an index.
///
/// It replaces the low discrepancy sequence when the caller asks for the
/// pseudorandom look. The index is mixed thoroughly so that nearby points
/// receive unrelated values. A plain linear congruential step is not enough:
/// it stays almost linear over a run of consecutive indices, which produces a
/// visible drift instead of a random spread. The mixing below is the finalizer
/// of the splitmix64 generator, and the result is reproducible across runs.
fn pseudorandom_unit(index: u64) -> f64 {
    let mut z = index.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;

    // Keep the top 53 bits, which is the precision of an f64 mantissa.
    ((z >> 11) as f64) / ((1u64 << 53) as f64)
}

/// Centers a list of signed offsets and stretches it to span `-1..1`.
///
/// A raw quasirandom sequence starts at an extreme, so without this step a
/// lane that holds a single point would push it to one edge instead of keeping
/// it in place. Centering makes small lanes symmetric, and stretching makes
/// them use the full lane width.
fn center_and_scale(sides: &mut [f64]) {
    if sides.is_empty() {
        return;
    }

    let mean = sides.iter().sum::<f64>() / sides.len() as f64;
    for side in sides.iter_mut() {
        *side -= mean;
    }

    let extent = sides.iter().map(|s| s.abs()).fold(0.0_f64, f64::max);
    if extent > 0.0 {
        for side in sides.iter_mut() {
            *side /= extent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn van_der_corput_starts_with_a_balanced_sequence() {
        let values: Vec<f64> = (0..4).map(van_der_corput).collect();
        assert_eq!(values, vec![0.0, 0.5, 0.25, 0.75]);
    }

    #[test]
    fn pseudorandom_unit_stays_in_range_and_is_repeatable() {
        let first = pseudorandom_unit(7);
        assert!((0.0..1.0).contains(&first));
        assert_eq!(first, pseudorandom_unit(7));
    }

    #[test]
    fn pseudorandom_unit_does_not_drift_for_neighbouring_indices() {
        // A linear congruential step would make consecutive values creep in one
        // direction. A proper hash jumps around the whole interval instead.
        let values: Vec<f64> = (0..16).map(pseudorandom_unit).collect();
        let below = values.iter().filter(|&&v| v < 0.5).count();
        assert!(
            (4..=12).contains(&below),
            "values should cover both halves: {values:?}"
        );
        assert!(
            values.windows(2).any(|w| w[0] < w[1]) && values.windows(2).any(|w| w[0] > w[1]),
            "values should not move in one direction: {values:?}"
        );
    }

    #[test]
    fn a_single_point_pairing_stays_centered() {
        let mut sides = vec![-1.0];
        center_and_scale(&mut sides);
        assert_eq!(sides, vec![0.0]);
    }

    #[test]
    fn a_two_point_pairing_is_symmetric() {
        let mut sides = vec![-1.0, 0.0];
        center_and_scale(&mut sides);
        assert!((sides[0] + sides[1]).abs() < 1e-12);
        assert!((sides[0] + 1.0).abs() < 1e-12);
        assert!((sides[1] - 1.0).abs() < 1e-12);
    }
}
