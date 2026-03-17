use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, PathConfig, RenderBackend};
use crate::error::ChartonError;
use crate::mark::line::MarkLine;
use crate::visual::color::SingleColor;
use polars::prelude::*;

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
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        if df_source.df.height() == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkLine configuration is missing".to_string()))?;

        // 1. Determine Grouping
        // We partition the dataframe so each group (e.g., "Sine", "Cosine") is a separate line.
        let group_column = context
            .spec
            .aesthetics
            .color
            .as_ref()
            .map(|c| c.field.as_str());

        let groups = match group_column {
            Some(col_name) => df_source.df.partition_by([col_name], true)?,
            None => vec![df_source.df.clone()],
        };

        for group_df in groups {
            // 2. Resolve Aesthetics for this group
            let group_color = self.resolve_group_color(&group_df, context, &mark_config.color)?;

            // 3. Extract and Sort Coordinates
            // Lines must be sorted by X-axis to avoid zig-zagging artifacts.
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

            // We sort ascending by X-axis to ensure the line path flows correctly.
            let sorted_df = group_df.sort(
                [x_enc.field.as_str()],
                SortMultipleOptions::default()
                    .with_order_descending(false) // Ascending order
                    .with_nulls_last(true), // Keep valid data at the front
            )?;
            let x_series = sorted_df.column(&x_enc.field)?.as_materialized_series();
            let y_series = sorted_df.column(&y_enc.field)?.as_materialized_series();

            // Extract Series data and convert to Vec<f64> for statistical processing.
            // Using into_no_null_iter() assumes data has been pre-filtered for nulls
            // to maximize performance via contiguous memory (Vec).
            let x_vals: Vec<f64> = x_series.f64()?.into_no_null_iter().collect();
            let y_vals: Vec<f64> = y_series.f64()?.into_no_null_iter().collect();

            // 4. Apply LOESS (Locally Estimated Scatterplot Smoothing) if enabled for this mark.
            // This is performed in data space (before normalization) to maintain statistical integrity.
            let (calc_x, calc_y) = if mark_config.loess {
                crate::stats::stat_loess::loess(&x_vals, &y_vals, mark_config.loess_bandwidth)
            } else {
                (x_vals, y_vals)
            };

            // 5. Normalize and Transform to Physical Pixels
            let x_scale_trait = context.coord.get_x_scale();
            let y_scale_trait = context.coord.get_y_scale();

            // Instead of converting to Series, we do a "Manual Vectorization"
            // This is just as fast as Polars' .apply() because it's a simple tight loop.
            let raw_points: Vec<(f64, f64)> = calc_x
                .into_iter()
                .zip(calc_y.into_iter())
                .map(|(x, y)| {
                    // Direct access to the normalization logic without Series overhead
                    let nx = x_scale_trait.normalize(x);
                    let ny = y_scale_trait.normalize(y);

                    context.transform(nx, ny)
                })
                .collect();

            if raw_points.is_empty() {
                continue;
            }

            // 6. Apply Interpolation Expansion
            let final_points = match mark_config.interpolation {
                PathInterpolation::Linear => raw_points,
                PathInterpolation::StepAfter => self.expand_step_after(raw_points),
                PathInterpolation::StepBefore => self.expand_step_before(raw_points),
            };
            let final_points = final_points
                .into_iter()
                .map(|(x, y)| (x as Precision, y as Precision))
                .collect();
            // 6. Dispatch to Backend
            backend.draw_path(PathConfig {
                points: final_points,
                stroke: group_color,
                stroke_width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
                dash: mark_config.dash.iter().map(|&d| d as Precision).collect(),
            });
        }

        Ok(())
    }
}

impl Chart<MarkLine> {
    /// Injects corner points for Step-After interpolation.
    fn expand_step_after(&self, points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        let mut expanded = Vec::with_capacity(points.len() * 2);
        for i in 0..points.len() - 1 {
            let (x1, y1) = points[i];
            let (x2, _y2) = points[i + 1];
            expanded.push((x1, y1));
            expanded.push((x2, y1)); // The "Step"
        }
        expanded.push(*points.last().unwrap());
        expanded
    }

    /// Injects corner points for Step-Before interpolation.
    fn expand_step_before(&self, points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        let mut expanded = Vec::with_capacity(points.len() * 2);
        for i in 0..points.len() - 1 {
            let (x1, y1) = points[i];
            let (_x2, y2) = points[i + 1];
            expanded.push((x1, y1));
            expanded.push((x1, y2)); // The "Step"
        }
        expanded.push(*points.last().unwrap());
        expanded
    }

    /// Resolves the color for a specific group of data.
    fn resolve_group_color(
        &self,
        df: &DataFrame,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            // Since all points in a group share the same category, we just map the first value
            let first_val = s_trait
                .scale_type()
                .normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = first_val.get(0).unwrap_or(0.0);
            Ok(s_trait
                .mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| *fallback))
        } else {
            Ok(*fallback)
        }
    }
}
