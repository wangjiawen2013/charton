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
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding is missing".into()))?;
        let y_field = self
            .encoding
            .y
            .as_ref()
            .map(|y| y.field.as_str())
            .ok_or_else(|| ChartonError::Encoding("Y-axis encoding is missing".into()))?;

        // Determine if we are in Manual Mode (y + y2) or Auto Mode (statistical suffix)
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
            .ok_or_else(|| ChartonError::Mark("MarkErrorBar configuration is missing".into()))?;

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

        // Normalize center points only if required (Auto Mode + show_center enabled)
        let yc_norms = if !is_manual_range && mark_config.show_center {
            Some(
                y_scale
                    .scale_type()
                    .normalize_column(y_scale, ds.column(y_field)?),
            )
        } else {
            None
        };

        // Resolve aesthetic color mapping if present
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

        // --- STEP 3: DODGING PREPARATION ---
        // We use pre-computed helper columns injected during transform_errorbar_data.
        // This ensures ErrorBars align perfectly with Boxplots even with missing data.
        let sub_idx_col = ds.column(&format!("{}_sub_idx", TEMP_SUFFIX))?;
        let groups_count_col = ds.column(&format!("{}_groups_count", TEMP_SUFFIX))?;

        let is_flipped = context.coord.is_flipped();

        // Calculate the step size for one unit on the X scale to handle dodging widths
        let unit_step_norm = (x_scale.normalize(1.0) - x_scale.normalize(0.0)).abs();

        // --- STEP 4: RENDERING LOOP ---
        // We iterate linearly through rows. The Cartesian product ensures NAN rows
        // exist for missing groups, preserving the correct visual "slot" (gap).
        for idx in 0..ds.row_count {
            let Some(xn) = x_norms[idx] else {
                continue;
            };

            // Get dodging metadata for the current row
            let sub_idx = sub_idx_col.get_f64(idx).unwrap_or(0.0);
            let n_groups = groups_count_col.get_f64(idx).unwrap_or(1.0);

            // Calculate horizontal offset (Dodge) using the same logic as Boxplot
            let actual_width =
                mark_config.span / (n_groups + (n_groups - 1.0) * mark_config.spacing);
            let width_norm = actual_width.min(mark_config.width) * unit_step_norm;
            let spacing_norm = width_norm * mark_config.spacing;
            let offset_norm = (sub_idx - (n_groups - 1.0) / 2.0) * (width_norm + spacing_norm);

            let x_final_n = xn + offset_norm;

            // Resolve color for this specific data point/group
            let mark_color = if let Some(ref norms) = color_norms {
                self.resolve_color_from_value(norms[idx], context, &mark_config.color)
            } else {
                mark_config.color
            };

            // 4.1 Draw Whisker & Caps
            // Block is skipped if values are NAN, creating a gap but keeping the slot reserved.
            if let (Some(yn1), Some(yn2)) = (y_min_norms[idx], y_max_norms[idx]) {
                let (x_pix1, y_pix1) = context.coord.transform(x_final_n, yn1, &context.panel);
                let (x_pix2, y_pix2) = context.coord.transform(x_final_n, yn2, &context.panel);

                // Draw vertical (or horizontal if flipped) whisker line
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

                // Draw Caps at both ends of the whisker
                let cap_len = mark_config.cap_length as Precision;
                if !is_flipped {
                    for py in [y_pix1 as Precision, y_pix2 as Precision] {
                        backend.draw_line(LineConfig {
                            x1: x_pix1 as Precision - cap_len,
                            y1: py,
                            x2: x_pix1 as Precision + cap_len,
                            y2: py,
                            color: mark_config.color,
                            width: mark_config.stroke_width as Precision,
                            opacity: mark_config.opacity as Precision,
                            dash: vec![],
                        });
                    }
                } else {
                    for px in [x_pix1 as Precision, x_pix2 as Precision] {
                        backend.draw_line(LineConfig {
                            x1: px,
                            y1: y_pix1 as Precision - cap_len,
                            x2: px,
                            y2: y_pix1 as Precision + cap_len,
                            color: mark_config.color,
                            width: mark_config.stroke_width as Precision,
                            opacity: mark_config.opacity as Precision,
                            dash: vec![],
                        });
                    }
                }
            }

            // 4.2 Draw Center Point (e.g., Mean or Median)
            if let Some(ref center_norms) = yc_norms {
                if let Some(ycn) = center_norms[idx] {
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
        Ok(())
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
