use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RenderBackend, TextConfig};
use crate::core::utils::Parallelizable;
use crate::error::ChartonError;
use crate::mark::text::MarkText;
use crate::visual::color::SingleColor;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (Text implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkText> {
    /// Renders text marks by mapping data points to text elements in the backend.
    /// Uses group-based parallel processing for performance and deterministic Z-index.
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
            .ok_or_else(|| ChartonError::Mark("MarkText configuration is missing".into()))?;

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

        // --- STEP 4: MULTI-CORE PROCESSING PER GROUP ---
        for (group_idx, (_name, row_indices)) in grouped_data.groups.iter().enumerate() {
            let base_group_color = if color_field.is_some() {
                palette.get_color(group_idx)
            } else {
                mark_config.color
            };

            // Calculate text configurations in parallel
            let render_configs: Vec<TextConfig> = row_indices
                .maybe_par_iter()
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

                    // Resolve Text Content
                    let content = if let Some(ref text_enc) = self.encoding.text {
                        df_source
                            .column(&text_enc.field)
                            .ok()?
                            .get_as_string(i)
                            .unwrap_or_default()
                    } else {
                        mark_config.text.clone()
                    };

                    Some(TextConfig {
                        x: px as Precision,
                        y: py as Precision,
                        text: content,
                        font_size: mark_config.font_size as Precision,
                        font_family: mark_config.font_family.clone(),
                        color: fill,
                        text_anchor: mark_config.text_anchor.to_string(),
                        font_weight: mark_config.font_weight.to_string(),
                        opacity: mark_config.opacity as Precision,
                    })
                })
                .collect();

            // --- STEP 5: SEQUENTIAL DRAW ---
            for config in render_configs {
                backend.draw_text(config);
            }
        }

        Ok(())
    }
}

impl Chart<MarkText> {
    /// Re-using the optimized color resolution logic.
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
