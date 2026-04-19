use crate::Precision;
use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{CircleConfig, LineConfig, MarkRenderer, RenderBackend};
use crate::error::ChartonError;
use crate::mark::errorbar::MarkErrorBar;
use crate::visual::color::SingleColor;

// ============================================================================
// MARK RENDERING (ErrorBar Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkErrorBar> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let ds = &self.data;
        if ds.row_count == 0 {
            return Ok(());
        }

        // --- STEP 1: RESOLVE FIELDS & CONFIG ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis missing".into()))?;
        let y_field = self
            .encoding
            .y
            .as_ref()
            .map(|y| y.field.as_str())
            .ok_or_else(|| ChartonError::Encoding("Y-axis missing".into()))?;
        let is_manual_range = self.encoding.y2.is_some();

        let (y_min_field, y_max_field) = if let Some(y2) = &self.encoding.y2 {
            (y_field.to_string(), y2.field.clone())
        } else {
            (
                format!("{}_{}_min", TEMP_SUFFIX, y_field),
                format!("{}_{}_max", TEMP_SUFFIX, y_field),
            )
        };

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkErrorBar config missing".into()))?;

        // --- STEP 2: VECTORIZED NORMALIZATION ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_min_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_min_field)?);
        let y_max_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_max_field)?);

        let yc_norms = if !is_manual_range && mark_config.show_center {
            Some(
                y_scale
                    .scale_type()
                    .normalize_column(y_scale, ds.column(y_field)?),
            )
        } else {
            None
        };

        let color_norms = if let Some(ref mapping) = context.spec.aesthetics.color {
            let s_trait = mapping.scale_impl.as_ref();
            Some(
                s_trait
                    .scale_type()
                    .normalize_column(s_trait, ds.column(&mapping.field)?),
            )
        } else {
            None
        };

        // --- STEP 3: DODGING PREP (FALLBACK LOGIC) ---
        // Try to get pre-computed columns first (from transform_errorbar_data)
        let sub_idx_col = ds.column(&format!("{}_sub_idx", TEMP_SUFFIX)).ok();
        let groups_count_col = ds.column(&format!("{}_groups_count", TEMP_SUFFIX)).ok();

        let is_flipped = context.coord.is_flipped();
        let unit_step_norm = (x_scale.normalize(1.0) - x_scale.normalize(0.0)).abs();

        // --- STEP 4: RENDERING ---
        if let (Some(sub_col), Some(cnt_col)) = (sub_idx_col, groups_count_col) {
            // Path A: Optimized linear rendering (Auto-statistical mode)
            for (idx, xn_opt) in x_norms.iter().enumerate().take(ds.row_count) {
                let Some(xn) = *xn_opt else { continue };
                let sub_idx = sub_col.get_f64(idx).unwrap_or(0.0);
                let n_groups = cnt_col.get_f64(idx).unwrap_or(1.0);
                self.render_errorbar_item(
                    idx,
                    xn,
                    sub_idx,
                    n_groups,
                    unit_step_norm,
                    is_flipped,
                    &y_min_norms,
                    &y_max_norms,
                    &yc_norms,
                    &color_norms,
                    mark_config,
                    context,
                    backend,
                );
            }
        } else {
            // Path B: Fallback to group_by logic (Manual/transform_calculate mode)
            let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());
            let grouped_data = ds.group_by(color_field);
            let n_groups = grouped_data.groups.len() as f64;

            for (group_idx, (_group_key, row_indices)) in grouped_data.groups.iter().enumerate() {
                for &idx in row_indices {
                    let Some(xn) = x_norms[idx] else { continue };
                    let sub_idx = group_idx as f64;
                    self.render_errorbar_item(
                        idx,
                        xn,
                        sub_idx,
                        n_groups,
                        unit_step_norm,
                        is_flipped,
                        &y_min_norms,
                        &y_max_norms,
                        &yc_norms,
                        &color_norms,
                        mark_config,
                        context,
                        backend,
                    );
                }
            }
        }

        Ok(())
    }
}

// Keep the core drawing logic in one place to avoid code duplication between Path A and B
impl Chart<MarkErrorBar> {
    #[allow(clippy::too_many_arguments)]
    fn render_errorbar_item(
        &self,
        idx: usize,
        xn: f64,
        sub_idx: f64,
        n_groups: f64,
        unit_step_norm: f64,
        is_flipped: bool,
        y_min_norms: &[Option<f64>],
        y_max_norms: &[Option<f64>],
        yc_norms: &Option<Vec<Option<f64>>>,
        color_norms: &Option<Vec<Option<f64>>>,
        mark_config: &MarkErrorBar,
        context: &PanelContext,
        backend: &mut dyn RenderBackend,
    ) {
        // --- 1. Calculate Dodge Offset ---
        let offset_norm = if n_groups > 1.0 {
            let actual_width =
                mark_config.span / (n_groups + (n_groups - 1.0) * mark_config.spacing);
            let width_norm = actual_width.min(mark_config.width) * unit_step_norm;
            let spacing_norm = width_norm * mark_config.spacing;
            (sub_idx - (n_groups - 1.0) / 2.0) * (width_norm + spacing_norm)
        } else {
            0.0
        };

        let x_final_n = xn + offset_norm;
        let mark_color = if let Some(norms) = color_norms {
            self.resolve_color_from_value(norms[idx], context, &mark_config.color)
        } else {
            mark_config.color
        };

        // --- 2. Draw Main Whisker and Caps ---
        if let (Some(yn1), Some(yn2)) = (y_min_norms[idx], y_max_norms[idx]) {
            // Transform both endpoints to pixel coordinates
            let (x_pix1, y_pix1) = context.coord.transform(x_final_n, yn1, &context.panel);
            let (x_pix2, y_pix2) = context.coord.transform(x_final_n, yn2, &context.panel);

            // 2.1 Draw the main connecting line (Whisker)
            backend.draw_line(LineConfig {
                x1: x_pix1 as Precision,
                y1: y_pix1 as Precision,
                x2: x_pix2 as Precision,
                y2: y_pix2 as Precision,
                color: mark_config.color,
                width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
                dash: vec![],
            });

            // 2.2 Draw Caps at both points
            let cap_len = mark_config.cap_length as Precision;
            let endpoints = [(x_pix1, y_pix1), (x_pix2, y_pix2)];

            for (px, py) in endpoints {
                let (x1, y1, x2, y2);
                if !is_flipped {
                    // Vertical ErrorBar: Cap is horizontal
                    x1 = px as Precision - cap_len;
                    y1 = py as Precision;
                    x2 = px as Precision + cap_len;
                    y2 = py as Precision;
                } else {
                    // Horizontal ErrorBar: Cap is vertical
                    x1 = px as Precision;
                    y1 = py as Precision - cap_len;
                    x2 = px as Precision;
                    y2 = py as Precision + cap_len;
                }

                backend.draw_line(LineConfig {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: mark_config.color,
                    width: mark_config.stroke_width as Precision,
                    opacity: mark_config.opacity as Precision,
                    dash: vec![],
                });
            }
        }

        // --- 3. Draw Center Point ---
        if let Some(center_norms) = yc_norms
            && let Some(ycn) = center_norms[idx]
        {
            let (cx, cy) = context.coord.transform(x_final_n, ycn, &context.panel);
            backend.draw_circle(CircleConfig {
                x: cx as Precision,
                y: cy as Precision,
                radius: 3.0,
                fill: mark_color,
                stroke: mark_color,
                stroke_width: 0.0,
                opacity: mark_config.opacity as Precision,
            });
        }
    }
}

impl Chart<MarkErrorBar> {
    /// Maps a normalized aesthetic value to a concrete color using the scale's palette.
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
