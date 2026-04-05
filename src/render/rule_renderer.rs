use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{LineConfig, MarkRenderer, RenderBackend};
use crate::error::ChartonError;
use crate::mark::rule::MarkRule;
use crate::visual::color::SingleColor;
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (Rule Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkRule> {
    /// Renders rule marks (straight lines spanning a specific range).
    /// Optimized with parallel geometry projection and deterministic grouping.
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
            .ok_or_else(|| ChartonError::Encoding("X encoding is missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y encoding is missing".into()))?;
        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkRule configuration is missing".into()))?;

        // --- STEP 2: POSITION & AESTHETIC NORMALIZATION ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, df_source.column(&x_enc.field)?);
        let y1_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, df_source.column(&y_enc.field)?);

        // Normalize Y2 if provided; otherwise we'll default to 1.0 during projection
        let y2_norms = self.encoding.y2.as_ref().map(|e| {
            y_scale
                .scale_type()
                .normalize_column(y_scale, &df_source.column(&e.field).unwrap())
        });

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
            let base_group_color = if color_field.is_some() {
                palette.get_color(group_idx)
            } else {
                mark_config.color
            };

            // Calculate rule geometries in parallel
            let render_configs: Vec<LineConfig> = row_indices
                .into_par_iter()
                .filter_map(|&i| {
                    let x_n = x_norms[i]?;
                    let yn1 = y1_norms[i]?;

                    // Requirement: Default to 1.0 if y2 is missing
                    let yn2 = y2_norms.as_ref().and_then(|ns| ns[i]).unwrap_or(1.0);

                    // Project endpoints to physical pixels
                    let (p1_x, p1_y) = context.coord.transform(x_n, yn1, &context.panel);
                    let (p2_x, p2_y) = context.coord.transform(x_n, yn2, &context.panel);

                    // Correct orientation based on coord_flip logic:
                    // Standard: Constant X, Y ranges from y1 to y2 (Vertical)
                    // Flipped: Constant Y, X ranges from x1 to x2 (Horizontal)
                    let (final_x1, final_y1, final_x2, final_y2) = if !is_flipped {
                        (p1_x, p1_y, p1_x, p2_y)
                    } else {
                        (p1_x, p1_y, p2_x, p1_y)
                    };

                    // Resolve color (Continuous scale mapping vs Group fallback)
                    let line_color = if let Some(ref norms) = color_norms {
                        self.resolve_color_from_value(norms[i], context, &base_group_color)
                    } else {
                        base_group_color
                    };

                    Some(LineConfig {
                        x1: final_x1 as Precision,
                        y1: final_y1 as Precision,
                        x2: final_x2 as Precision,
                        y2: final_y2 as Precision,
                        color: line_color,
                        width: mark_config.stroke_width as Precision,
                        opacity: mark_config.opacity as Precision,
                        dash: vec![],
                    })
                })
                .collect();

            // --- STEP 5: SEQUENTIAL DRAW ---
            for config in render_configs {
                backend.draw_line(config);
            }
        }

        Ok(())
    }
}

impl Chart<MarkRule> {
    /// Reusable aesthetic color resolver.
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
