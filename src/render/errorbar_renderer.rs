use crate::Precision;
use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{CircleConfig, LineConfig, MarkRenderer, RenderBackend};
use crate::error::ChartonError;
use crate::mark::errorbar::MarkErrorBar;
use crate::visual::color::SingleColor;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkErrorBar> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        if df_source.df.height() == 0 {
            return Ok(());
        }

        // --- STEP 1: RESOLVE FIELD NAMES & CONFIG ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding is missing".to_string()))?;
        let y_field = self
            .encoding
            .y
            .as_ref()
            .map(|y| y.field.as_str())
            .ok_or_else(|| ChartonError::Encoding("Y-axis encoding is missing".to_string()))?;

        let (y_min_field, y_max_field) = if let Some(y2) = &self.encoding.y2 {
            (y_field.to_string(), y2.field.clone())
        } else {
            (
                format!("{}_{}_min", TEMP_SUFFIX, y_field),
                format!("{}_{}_max", TEMP_SUFFIX, y_field),
            )
        };

        let mark_config = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Mark("MarkErrorBar configuration is missing".to_string())
        })?;

        // --- STEP 2: GROUPING & COORDINATE PREP ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();
        let is_flipped = context.coord.is_flipped();

        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());

        // Simplified logic: If a color field exists, it is guaranteed to be
        // different from X and discrete due to earlier validation/transform steps.
        let is_dodged = color_field.is_some();

        let groups = if let Some(field) = color_field {
            df_source.df.partition_by_stable([field], true)?
        } else {
            vec![df_source.df.clone()]
        };

        let n_groups = groups.len() as f64;
        // Calculate the normalized distance between two adjacent categories (the unit width of an X-axis step).
        let unit_step_norm = (x_scale.normalize(1.0) - x_scale.normalize(0.0)).abs();

        // --- STEP 3: RENDERING LOOP ---
        for (group_idx, group_df) in groups.iter().enumerate() {
            // Resolve Color for this specific group
            let group_color = self.resolve_group_color(group_df, context, &mark_config.color)?;

            // Calculate Dodge Offset
            let offset_norm = if is_dodged && n_groups > 1.0 {
                let actual_width =
                    mark_config.span / (n_groups + (n_groups - 1.0) * mark_config.spacing);
                let width_norm = actual_width.min(mark_config.width) * unit_step_norm;
                let spacing_norm = width_norm * mark_config.spacing;
                (group_idx as f64 - (n_groups - 1.0) / 2.0) * (width_norm + spacing_norm)
            } else {
                0.0
            };

            // Fetch and Normalize Data
            let x_series = group_df.column(&x_enc.field)?.as_materialized_series();
            let y_series = group_df.column(y_field)?.as_materialized_series();
            let y_min_series = group_df.column(&y_min_field)?.as_materialized_series();
            let y_max_series = group_df.column(&y_max_field)?.as_materialized_series();

            let x_norms = x_scale.scale_type().normalize_series(x_scale, x_series)?;
            let y_center_norms = y_scale.scale_type().normalize_series(y_scale, y_series)?;
            let y_min_norms = y_scale
                .scale_type()
                .normalize_series(y_scale, y_min_series)?;
            let y_max_norms = y_scale
                .scale_type()
                .normalize_series(y_scale, y_max_series)?;

            for (((x_n, yc_n), y_min_n), y_max_n) in x_norms
                .into_iter()
                .zip(y_center_norms.into_iter())
                .zip(y_min_norms.into_iter())
                .zip(y_max_norms.into_iter())
            {
                let Some(xn) = x_n else {
                    continue;
                };
                let x_final_n = xn + offset_norm;

                // 1. Draw Whisker & Caps (Only if n > 1, i.e., bounds are not Null)
                if let (Some(yn1), Some(yn2)) = (y_min_n, y_max_n) {
                    let (x_pix1, y_pix1) = context.transform(x_final_n, yn1);
                    let (x_pix2, y_pix2) = context.transform(x_final_n, yn2);

                    backend.draw_line(LineConfig {
                        x1: x_pix1 as Precision,
                        y1: y_pix1 as Precision,
                        x2: x_pix2 as Precision,
                        y2: y_pix2 as Precision,
                        color: mark_config.color,
                        width: mark_config.stroke_width as Precision,
                        opacity: mark_config.opacity as Precision,
                        dash: None,
                    });

                    // Caps Logic
                    let cap_len = mark_config.cap_length as Precision;
                    let (px1, py1) = (x_pix1 as Precision, y_pix1 as Precision);
                    let (px2, py2) = (x_pix2 as Precision, y_pix2 as Precision);

                    if !is_flipped {
                        for py in [py1, py2] {
                            backend.draw_line(LineConfig {
                                x1: px1 - cap_len,
                                y1: py,
                                x2: px1 + cap_len,
                                y2: py,
                                color: mark_config.color,
                                width: mark_config.stroke_width as Precision,
                                opacity: mark_config.opacity as Precision,
                                dash: None,
                            });
                        }
                    } else {
                        for px in [px1, px2] {
                            backend.draw_line(LineConfig {
                                x1: px,
                                y1: py1 - cap_len,
                                x2: px,
                                y2: py1 + cap_len,
                                color: mark_config.color,
                                width: mark_config.stroke_width as Precision,
                                opacity: mark_config.opacity as Precision,
                                dash: None,
                            });
                        }
                    }
                }

                // 2. Draw Center Point
                if let (true, Some(ycn)) = (mark_config.show_center, yc_n) {
                    let (cx, cy) = context.transform(x_final_n, ycn);

                    backend.draw_circle(CircleConfig {
                        x: cx as Precision,
                        y: cy as Precision,
                        radius: 3.0,
                        fill: group_color,
                        stroke: group_color,
                        stroke_width: 0.0,
                        opacity: mark_config.opacity as Precision,
                    });
                }
            }
        }
        Ok(())
    }
}

impl Chart<MarkErrorBar> {
    /// Resolves the color for a specific data group in ErrorBar.
    /// This ensures ErrorBars use the same color palette as Bars.
    fn resolve_group_color(
        &self,
        df: &DataFrame,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();

            // Normalize the first value to find the group's color
            let norms = s_trait
                .scale_type()
                .normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = norms.get(0).unwrap_or(0.0);

            // Map via the scale's mapper (Palette)
            Ok(s_trait
                .mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| *fallback))
        } else {
            Ok(*fallback)
        }
    }
}
