use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RectConfig, RenderBackend};
use crate::error::ChartonError;
use crate::mark::tick::MarkTick;
use crate::visual::color::SingleColor;
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (Tick Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkTick> {
    /// Renders tick marks by transforming data points into thin rectangular geometries.
    /// Uses group-based parallel processing to ensure deterministic rendering and performance.
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

        // --- STEP 1: VALIDATION ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding is missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y-axis encoding is missing".into()))?;
        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkTick configuration is missing".into()))?;

        // --- STEP 2: POSITION & AESTHETIC NORMALIZATION ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, df_source.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, df_source.column(&y_enc.field)?);

        // Pre-normalize color if mapping exists
        let color_norms = context.spec.aesthetics.color.as_ref().map(|m| {
            let s = m.scale_impl.as_ref();
            s.scale_type()
                .normalize_column(s, &df_source.column(&m.field).unwrap())
        });

        // --- STEP 3: GROUPING (Determines Z-Index & Category Color) ---
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());
        let grouped_data = df_source.group_by(color_field);
        let palette = &context.spec.theme.palette;

        let is_flipped = context.coord.is_flipped();

        // --- STEP 4: MULTI-CORE PROCESSING PER GROUP ---
        for (group_idx, (_name, row_indices)) in grouped_data.groups.iter().enumerate() {
            // Resolve base color for this group
            let base_group_color = if color_field.is_some() {
                palette.get_color(group_idx)
            } else {
                mark_config.color
            };

            // Calculate tick geometries in parallel
            let render_configs: Vec<RectConfig> = row_indices
                .into_par_iter()
                .filter_map(|&i| {
                    let x_n = x_norms[i]?;
                    let y_n = y_norms[i]?;

                    // Coordinate Projection
                    let (px, py) = context.coord.transform(x_n, y_n, &context.panel);

                    // Resolve Color
                    let fill = if let Some(ref norms) = color_norms {
                        self.resolve_color_from_value(norms[i], context, &base_group_color)
                    } else {
                        base_group_color
                    };

                    // Tick Geometry Calculation
                    let thickness = mark_config.thickness;
                    let band_size = mark_config.band_size;

                    let (rx, ry, rw, rh) = if !is_flipped {
                        // Vertical ticks: narrow width, tall height
                        (
                            px - thickness / 2.0,
                            py - band_size / 2.0,
                            thickness,
                            band_size,
                        )
                    } else {
                        // Horizontal ticks: wide width, narrow height
                        (
                            px - band_size / 2.0,
                            py - thickness / 2.0,
                            band_size,
                            thickness,
                        )
                    };

                    Some(RectConfig {
                        x: rx as Precision,
                        y: ry as Precision,
                        width: rw as Precision,
                        height: rh as Precision,
                        fill,
                        stroke: fill, // Border matches fill for ticks
                        stroke_width: 0.0,
                        opacity: mark_config.opacity as Precision,
                    })
                })
                .collect();

            // --- STEP 5: SEQUENTIAL DRAW DISPATCH ---
            for config in render_configs {
                backend.draw_rect(config);
            }
        }

        Ok(())
    }
}

impl Chart<MarkTick> {
    /// Shared utility to map a normalized data value to its aesthetic color.
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
