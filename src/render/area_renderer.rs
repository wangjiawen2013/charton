//! Area geometry: the region between a line and its baseline.
//!
//! Rows are grouped by colour and split into *segments* wherever `x` or `y` is
//! missing, so a hole in the data opens the area instead of being bridged. Each
//! segment becomes one closed polygon (an upper boundary and the baseline run
//! back) plus, in overlay mode, an independent top stroke.

use crate::Precision;
use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, PathConfig, PathTopology, RenderBackend};
use crate::core::utils::Parallelizable;
use crate::encode::y::StackMode;
use crate::error::ChartonError;
use crate::mark::area::MarkArea;
use crate::visual::color::SingleColor;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl MarkRenderer for Chart<MarkArea> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let ds = &self.data;
        if ds.row_count == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkArea configuration is missing".to_string()))?;

        // --- STEP 1: Extract Encodings and Scales ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or(ChartonError::Encoding("Y missing".into()))?;
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // Identify temporary column names for stacked/stream modes
        let y_field = y_enc.field.as_str();
        let y0_field = format!("{}_{}_min", TEMP_SUFFIX, y_field);
        let y1_field = format!("{}_{}_max", TEMP_SUFFIX, y_field);

        let use_stacked = matches!(
            y_enc.stack,
            StackMode::Stacked | StackMode::Normalize | StackMode::Center | StackMode::Mirror
        );

        // --- STEP 2: Render Zero Baseline ---
        // Only rendered for unstacked modes to provide a visual reference for 0.0
        if !use_stacked {
            self.draw_zero_baseline(backend, context);
        }

        // --- STEP 3: Vectorized Column Extraction (Normalized Space) ---
        // Pre-normalize all required columns to [0.0, 1.0] for efficient parallel processing
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_enc.field)?);

        let y0_norms = if use_stacked {
            Some(
                y_scale
                    .scale_type()
                    .normalize_column(y_scale, ds.column(&y0_field)?),
            )
        } else {
            None
        };
        let y1_norms = if use_stacked {
            Some(
                y_scale
                    .scale_type()
                    .normalize_column(y_scale, ds.column(&y1_field)?),
            )
        } else {
            None
        };

        // Normalize color column if a mapping exists
        let color_norms = if let Some(ref color_map) = context.spec.aesthetics.color {
            Some(
                color_map
                    .scale_impl
                    .scale_type()
                    .normalize_column(color_map.scale_impl.as_ref(), ds.column(&color_map.field)?),
            )
        } else {
            None
        };

        // --- STEP 4: Grouping and Parallel Path Construction ---
        let color_field = context
            .spec
            .aesthetics
            .color
            .as_ref()
            .map(|c| c.field.as_str());
        let grouped_data = ds.group_by(color_field);

        let area_render_data: Vec<_> = grouped_data
            .groups
            .maybe_par_iter()
            .filter_map(|(_name, row_indices)| {
                if row_indices.is_empty() {
                    return None;
                }

                let build_point = |idx: usize| -> Option<AreaInternalPoint> {
                    let xn = x_norms[idx]?;
                    if use_stacked {
                        Some(AreaInternalPoint {
                            xn,
                            yn: y1_norms.as_ref()?[idx]?,
                            y0n: y0_norms.as_ref()?[idx]?,
                        })
                    } else {
                        Some(AreaInternalPoint {
                            xn,
                            yn: y_norms[idx]?,
                            y0n: 0.0, // Default baseline is 0.0 in normalized space for unstacked areas
                        })
                    }
                };

                // 4.1 Split into contiguous segments at missing x/y values,
                // so a data gap breaks the area instead of bridging it.
                let mut segments: Vec<Vec<AreaInternalPoint>> = Vec::new();
                let mut current: Vec<AreaInternalPoint> = Vec::new();
                for &idx in row_indices {
                    match build_point(idx) {
                        Some(point) => current.push(point),
                        None if !current.is_empty() => {
                            segments.push(std::mem::take(&mut current));
                        }
                        None => {}
                    }
                }
                if !current.is_empty() {
                    segments.push(current);
                }
                if segments.is_empty() {
                    return None;
                }

                // 4.2 Resolve group color using shared logic
                let first_idx = row_indices[0];
                // A present-but-null value is `Some(NaN)`: the shared resolver
                // paints it grey, while a layer without colour stays fallback.
                let color_val = color_norms
                    .as_ref()
                    .map(|cn| cn[first_idx].unwrap_or(f64::NAN));
                let group_color =
                    self.resolve_color_from_value(color_val, context, &mark_config.color);

                // 4.3 Sort, project and close each segment into a polygon. Sorting
                // keeps the polygon vertices monotonic for linear and ordinal axes.
                let segments: Vec<ProjectedSegment> = segments
                    .into_iter()
                    .map(|mut points| {
                        // Critical sort to prevent self-intersecting polygon rendering
                        points.sort_by(|a, b| {
                            a.xn.partial_cmp(&b.xn).unwrap_or(std::cmp::Ordering::Equal)
                        });

                        let mut fill_pts: Vec<(Precision, Precision)> =
                            Vec::with_capacity(points.len() * 2);
                        let mut stroke_pts: Vec<(Precision, Precision)> =
                            Vec::with_capacity(points.len());

                        // Build Upper Boundary Path (y1)
                        for p in &points {
                            let (px, py) = context.coord.transform(p.xn, p.yn, &context.panel);
                            let pt = (px as Precision, py as Precision);
                            fill_pts.push(pt);
                            stroke_pts.push(pt);
                        }

                        // Reverse build Lower Boundary Path (y0) to close the polygon
                        for p in points.iter().rev() {
                            let (px, py_base) =
                                context.coord.transform(p.xn, p.y0n, &context.panel);
                            fill_pts.push((px as Precision, py_base as Precision));
                        }

                        (fill_pts, stroke_pts)
                    })
                    .collect();

                Some((segments, group_color))
            })
            .collect();

        // --- STEP 5: Final Dispatch to Backend ---
        for (segments, group_color) in area_render_data {
            for (fill_pts, stroke_pts) in segments {
                // Layer 1: Area Fill (Unified concave polygon fill)
                // One closed path. `draw_path` with `PathTopology::Complex`
                // routes to the WGPU stencil-then-cover pipeline; SVG/PNG render
                // it as a standard closed vector path with anti-aliasing.
                backend.draw_path(PathConfig {
                    points: fill_pts,
                    fill: group_color,
                    stroke: SingleColor::none(),
                    stroke_width: 0.0,
                    opacity: mark_config.opacity as Precision,
                    dash: vec![],
                    topology: PathTopology::Complex,
                });

                // Layer 2: Top Boundary Path (Independent top stroke/polyline)
                // Note: Stacked modes usually omit strokes to prevent edge artifacts in streamgraphs
                if matches!(y_enc.stack, StackMode::None) {
                    backend.draw_path(PathConfig {
                        points: stroke_pts,
                        fill: SingleColor::none(),
                        stroke: group_color,
                        stroke_width: mark_config.stroke_width as Precision,
                        // Stroke opacity is usually kept at 1.0 to highlight the boundary,
                        // or explicitly defined by external styles.
                        opacity: 1.0,
                        dash: mark_config.dash.iter().map(|&d| d as Precision).collect(),
                        // Explicitly providing the Simple topology ensures this routes
                        // to the high-performance normal extrusion pipeline.
                        topology: PathTopology::Simple,
                    });
                }
            }
        }

        Ok(())
    }
}

// --- Internal Helper Structure ---

/// One closed area segment: the upper+lower fill polygon and its top stroke.
type ProjectedSegment = (Vec<(Precision, Precision)>, Vec<(Precision, Precision)>);

struct AreaInternalPoint {
    xn: f64,  // Normalized X
    yn: f64,  // Normalized Y (Top Boundary)
    y0n: f64, // Normalized Y0 (Baseline/Bottom Boundary)
}

impl Chart<MarkArea> {
    /// Renders a dashed reference line at y=0 if it falls within the current axis domain
    fn draw_zero_baseline(&self, backend: &mut dyn RenderBackend, context: &PanelContext) {
        let y_scale = context.coord.get_y_scale();
        let (y_min, y_max) = y_scale.domain();

        if y_min <= 0.0 && y_max >= 0.0 {
            let b_norm = y_scale.normalize(0.0);
            let (px1, py1) = context.coord.transform(0.0, b_norm, &context.panel);
            let (px2, py2) = context.coord.transform(1.0, b_norm, &context.panel);

            backend.draw_path(PathConfig {
                points: vec![
                    (px1 as Precision, py1 as Precision),
                    (px2 as Precision, py2 as Precision),
                ],
                fill: SingleColor::none(),
                stroke: SingleColor::from("#888888"),
                stroke_width: 1.0,
                opacity: 0.5,
                dash: vec![4.0, 4.0],
                topology: PathTopology::Simple,
            });
        }
    }

    /// Resolves this mark's colour; see [`crate::render::resolve_color`].
    fn resolve_color_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> SingleColor {
        crate::render::resolve_color(val, context, fallback)
    }
}
