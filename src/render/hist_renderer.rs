use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RectConfig, RenderBackend};
use crate::core::utils::Parallelizable;
use crate::error::ChartonError;
use crate::mark::histogram::MarkHist;
use crate::visual::color::SingleColor;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (Histogram Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkHist> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let ds = &self.data;
        if ds.height() == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkHist configuration is missing".into()))?;

        // --- STEP 1: RESOLVE SCALES & NORMALIZATION ---
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

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();
        let is_flipped = context.coord.is_flipped();

        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_enc.field)?);

        let color_norms = context.spec.aesthetics.color.as_ref().map(|m| {
            let s = m.scale_impl.as_ref();
            s.scale_type()
                .normalize_column(s, ds.column(&m.field).unwrap())
        });

        // --- STEP 2: GROUPING ---
        // Consistent with MarkLine: group by color field to handle overlaps and Z-indexing
        let group_field = context.spec.aesthetics.color.as_ref().map(|c| &c.field);
        let grouped_indices = ds.group_by(group_field.map(|s| s.as_str()));

        // --- STEP 3: GEOMETRY CALCULATION ---
        let bar_thickness = self.calculate_hist_bar_size(context)?;
        let y_baseline_norm = 0.0;

        // --- STEP 4: PARALLEL PROCESSING PER GROUP ---
        // We calculate all rects for all groups in parallel while maintaining the group structure
        let groups_render_data: Vec<Vec<RectConfig>> = grouped_indices
            .groups
            .maybe_par_iter()
            .map(|(_group_key, row_indices)| {
                row_indices
                    .iter()
                    .filter_map(|&idx| {
                        let x_n = x_norms[idx]?;
                        let y_n = y_norms[idx]?;

                        // Resolve color using unified logic
                        let fill = self.resolve_color_from_value(
                            color_norms.as_ref().and_then(|n| n[idx]),
                            context,
                            &mark_config.color,
                        );

                        let (px, py) = context.coord.transform(x_n, y_n, &context.panel);
                        let (px_base, py_base) =
                            context
                                .coord
                                .transform(x_n, y_baseline_norm, &context.panel);

                        Some(if !is_flipped {
                            let h = (py_base - py).abs();
                            RectConfig {
                                x: (px - bar_thickness / 2.0) as Precision,
                                y: py.min(py_base) as Precision,
                                width: bar_thickness as Precision,
                                height: h as Precision,
                                fill,
                                stroke: mark_config.stroke,
                                stroke_width: mark_config.stroke_width as Precision,
                                opacity: mark_config.opacity as Precision,
                            }
                        } else {
                            let w = (px - px_base).abs();
                            RectConfig {
                                x: px.min(px_base) as Precision,
                                y: (py - bar_thickness / 2.0) as Precision,
                                width: w as Precision,
                                height: bar_thickness as Precision,
                                fill,
                                stroke: mark_config.stroke,
                                stroke_width: mark_config.stroke_width as Precision,
                                opacity: mark_config.opacity as Precision,
                            }
                        })
                    })
                    .collect()
            })
            .collect();

        // --- STEP 5: SEQUENTIAL EMISSION ---
        // Iterate through groups in their original order to ensure correct Z-order layering
        for rects in groups_render_data {
            for config in rects {
                backend.draw_rect(config);
            }
        }

        Ok(())
    }
}

// --- HELPER METHODS ---

impl Chart<MarkHist> {
    /// Determines the pixel thickness of bars by measuring the distance
    /// between adjacent bin centers in the current coordinate system.
    fn calculate_hist_bar_size(&self, context: &PanelContext) -> Result<f64, ChartonError> {
        let x_enc = self.encoding.x.as_ref().unwrap();
        let n_bins = x_enc
            .bins
            .ok_or_else(|| ChartonError::Encoding("Bin count not resolved".into()))?
            as f64;

        let x_scale = context.coord.get_x_scale();
        let col = self.data.column(&x_enc.field)?;

        // --- OPTIMIZED: Use your parallel min_max method ---
        // This replaces two separate .min() and .max() calls with one parallel scan.
        let (v_min, v_max) = col.min_max();

        // Handle the case where the column might be effectively empty or invalid
        if v_min == f64::INFINITY || v_max == f64::NEG_INFINITY {
            return Err(ChartonError::Data(
                "X column is empty or contains only nulls".into(),
            ));
        }

        // Calculate logical data-space step between bins
        let data_step = if n_bins > 1.0 {
            (v_max - v_min) / (n_bins - 1.0)
        } else {
            // Fallback for single bin
            let (d0, d1) = x_scale.domain();
            (d1 - d0) * 0.5
        };

        // Map logical step to normalized space
        let norm0 = x_scale.normalize(v_min);
        let norm1 = x_scale.normalize(v_min + data_step);

        // Convert normalized span to pixels
        let (p0_x, p0_y) = context.coord.transform(norm0, 0.0, &context.panel);
        let (p1_x, p1_y) = context.coord.transform(norm1, 0.0, &context.panel);

        let theoretical_thickness = if context.coord.is_flipped() {
            (p1_y - p0_y).abs()
        } else {
            (p1_x - p0_x).abs()
        };

        // Apply a visual gap factor (0.95) to separate bars slightly
        Ok(theoretical_thickness * 0.95)
    }

    /// Resolves a SingleColor from a normalized aesthetic value.
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
}
