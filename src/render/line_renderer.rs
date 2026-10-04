//! Line geometry: group rows by colour, order them along x, and connect them.
//!
//! A group is drawn as one or more open paths. Rows are taken in the order they
//! appear, so a missing `x` or `y` splits the group into *segments*: the line is
//! interrupted instead of bridging the hole. That makes an outage in a time
//! series visible at a glance. Each segment is then sorted along x, optionally
//! smoothed, and projected independently.

use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, PathConfig, PathTopology, RenderBackend};
use crate::core::utils::Parallelizable;
use crate::error::ChartonError;
use crate::mark::line::MarkLine;
use crate::visual::color::SingleColor;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Interpolation methods for line paths
#[derive(Debug, Clone, Default)]
pub enum PathInterpolation {
    /// Straight line segments between points (default)
    #[default]
    Linear,
    /// Step function that holds value until next point (appropriate for ECDF)
    StepAfter,
    /// Step function that jumps to next value immediately
    StepBefore,
}

/// Implements conversion from string slices to `PathInterpolation`.
///
/// This enables a more ergonomic Fluent API, allowing users to pass string literals
/// like `.interpolation("step")` instead of the more verbose `PathInterpolation::StepAfter`.
impl From<&str> for PathInterpolation {
    /// Performs the conversion.
    ///
    /// # Arguments
    /// * `s` - A string slice representing the interpolation method (case-insensitive).
    fn from(s: &str) -> Self {
        // Convert to lowercase to ensure the API is case-insensitive (e.g., "Linear" vs "linear").
        match s.to_lowercase().as_str() {
            // Step-after: The value changes at the next data point.
            // Often used for step functions or ECDF visualizations.
            "step" | "step-after" => PathInterpolation::StepAfter,

            // Step-before: The value changes immediately at the current data point.
            "step-before" => PathInterpolation::StepBefore,

            // Linear: Simple straight line segments between data points (Standard).
            "linear" => PathInterpolation::Linear,

            // Fallback: If the input string is unrecognized, default to Linear interpolation
            // to ensure the rendering pipeline does not fail.
            _ => PathInterpolation::Linear,
        }
    }
}

// ============================================================================
// MARK RENDERING
// ============================================================================
impl MarkRenderer for Chart<MarkLine> {
    /// Transforms grouped raw data into connected paths (lines).
    /// Handles aesthetics, statistical smoothing (LOESS), and interpolation
    /// while maintaining Z-index order based on data appearance.
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
            .ok_or_else(|| ChartonError::Mark("MarkLine configuration is missing".into()))?;

        // --- STEP 1: SPECIFICATION VALIDATION & SCALING ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X is missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y is missing".into()))?;

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // Vectorized normalization of primary coordinates
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_enc.field)?);

        // Pre-normalize color column if a mapping exists (handles both Discrete and Continuous)
        let color_norms = context.spec.aesthetics.color.as_ref().and_then(|m| {
            let s = m.scale_impl.as_ref();
            let col = ds.column(&m.field).ok()?;
            Some(s.scale_type().normalize_column(s, col))
        });

        // --- STEP 2: GROUPING (Determining Path Separation) ---
        // Groups are sorted by "First Appearance" to ensure deterministic Z-indexing.
        let group_field = context.spec.aesthetics.color.as_ref().map(|c| &c.field);
        let grouped_indices = ds.group_by(group_field.map(|s| s.as_str()));

        // --- STEP 3: PARALLEL PATH CALCULATION ---
        let line_render_data: Vec<_> = grouped_indices
            .groups
            .maybe_par_iter()
            .filter_map(|(_group_key, row_indices)| {
                let first_idx = *row_indices.first()?;

                // 3.1 Split into contiguous segments at missing x/y values. A
                // gap in the data breaks the line rather than being bridged, so
                // e.g. a failed sensor interval is visible at a glance.
                let mut segments: Vec<Vec<(f64, f64)>> = Vec::new();
                let mut current: Vec<(f64, f64)> = Vec::new();
                for &idx in row_indices {
                    match (x_norms[idx], y_norms[idx]) {
                        (Some(xn), Some(yn)) => current.push((xn, yn)),
                        _ if !current.is_empty() => {
                            segments.push(std::mem::take(&mut current));
                        }
                        _ => {}
                    }
                }
                if !current.is_empty() {
                    segments.push(current);
                }
                if segments.is_empty() {
                    return None;
                }

                // 3.2 Unified Aesthetic Resolution:
                // We resolve the color based on the first point's normalized value.
                // This ensures symmetry with PointMark behavior.
                let final_color = self.resolve_color_from_value(
                    color_norms
                        .as_ref()
                        .map(|n| n[first_idx].unwrap_or(f64::NAN)),
                    context,
                    &mark_config.color,
                );

                // 3.3 Per-segment sort, smoothing, projection and interpolation.
                let segments: Vec<Vec<(f64, f64)>> = segments
                    .into_iter()
                    .map(|mut points| {
                        // Sorting: Ensure line monotonicity along the X-axis
                        points.sort_by(|a, b| {
                            a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
                        });

                        // Statistical Smoothing: Optional LOESS processing
                        let proc_points = if mark_config.loess {
                            let xs: Vec<f64> = points.iter().map(|p| p.0).collect();
                            let ys: Vec<f64> = points.iter().map(|p| p.1).collect();
                            let (lx, ly) = crate::stats::stat_loess::loess(
                                &xs,
                                &ys,
                                mark_config.loess_bandwidth,
                            );
                            lx.into_iter().zip(ly).collect()
                        } else {
                            points
                        };

                        // Projection: Convert normalized coordinates to pixel space
                        let projected: Vec<(f64, f64)> = proc_points
                            .into_iter()
                            .map(|(xn, yn)| context.coord.transform(xn, yn, &context.panel))
                            .collect();

                        // Interpolation: Expand points for Step-before/after paths
                        match mark_config.interpolation {
                            PathInterpolation::Linear => projected,
                            PathInterpolation::StepAfter => self.expand_step_after(projected),
                            PathInterpolation::StepBefore => self.expand_step_before(projected),
                        }
                    })
                    .collect();

                Some((segments, final_color))
            })
            .collect();

        // --- STEP 4: SEQUENTIAL DRAW DISPATCH ---
        // Lines are drawn in sequence to respect the Z-order established by grouping.
        // Each data gap is its own path, so the line is visibly interrupted.
        for (segments, color) in line_render_data {
            for points in segments {
                if points.is_empty() {
                    continue;
                }

                backend.draw_path(PathConfig {
                    points: points
                        .into_iter()
                        .map(|(px, py)| (px as Precision, py as Precision))
                        .collect(),
                    fill: "none".into(),
                    stroke: color,
                    stroke_width: mark_config.stroke_width as Precision,
                    opacity: mark_config.opacity as Precision,
                    dash: mark_config.dash.iter().map(|&d| d as Precision).collect(),
                    topology: PathTopology::Simple,
                });
            }
        }

        Ok(())
    }
}

impl Chart<MarkLine> {
    /// Injects corner points for Step-After interpolation.
    /// Capacity is pre-allocated to avoid reallocations.
    fn expand_step_after(&self, points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        if points.len() < 2 {
            return points;
        }
        let mut expanded = Vec::with_capacity(points.len() * 2);
        for i in 0..points.len() - 1 {
            let (x1, y1) = points[i];
            let (x2, _) = points[i + 1];
            expanded.push((x1, y1));
            expanded.push((x2, y1));
        }
        expanded.push(*points.last().unwrap());
        expanded
    }

    fn expand_step_before(&self, points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        if points.len() < 2 {
            return points;
        }
        let mut expanded = Vec::with_capacity(points.len() * 2);
        for i in 0..points.len() - 1 {
            let (x1, y1) = points[i];
            let (_, y2) = points[i + 1];
            expanded.push((x1, y1));
            expanded.push((x1, y2));
        }
        expanded.push(*points.last().unwrap());
        expanded
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
