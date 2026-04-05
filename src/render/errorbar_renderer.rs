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
    /// Renders error bars which typically represent variability (min/max/center).
    /// Supports "dodging" (side-by-side positioning) for grouped data.
    /// Renders error bars which typically represent variability (min/max/center).
    /// Supports "dodging" (side-by-side positioning) for grouped data.
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

        // Error bars require a range. Use explicit 'y2' or look for auto-generated min/max columns.
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
        let yc_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(y_field)?);
        let y_min_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_min_field)?);
        let y_max_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_max_field)?);

        // Resolve aesthetic color normalization if mapping exists.
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

        // --- STEP 3: GROUPING & DODGING PREP ---
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());
        let grouped_data = ds.group_by(color_field);
        let n_groups = grouped_data.groups.len() as f64;
        let is_flipped = context.coord.is_flipped();

        // unit_step_norm represents the logical width of one category on the X-axis.
        let unit_step_norm = (x_scale.normalize(1.0) - x_scale.normalize(0.0)).abs();

        // --- STEP 4: RENDERING LOOP (Organized by Group for Z-Index) ---
        for (group_idx, (_group_key, row_indices)) in grouped_data.groups.iter().enumerate() {
            if row_indices.is_empty() {
                continue;
            }

            // 4.1 Dodge Offset: Shift bars horizontally/vertically so groups don't overlap.
            let offset_norm = if color_field.is_some() && n_groups > 1.0 {
                let actual_width =
                    mark_config.span / (n_groups + (n_groups - 1.0) * mark_config.spacing);
                let width_norm = actual_width.min(mark_config.width) * unit_step_norm;
                let spacing_norm = width_norm * mark_config.spacing;
                (group_idx as f64 - (n_groups - 1.0) / 2.0) * (width_norm + spacing_norm)
            } else {
                0.0
            };

            for &idx in row_indices {
                // Access pre-computed normalized coordinates.
                let Some(xn) = x_norms[idx] else {
                    continue;
                };
                let x_final_n = xn + offset_norm;

                // 4.2 Draw Whisker & Caps (The range indicator)
                if let (Some(yn1), Some(yn2)) = (y_min_norms[idx], y_max_norms[idx]) {
                    let (x_pix1, y_pix1) = context.coord.transform(x_final_n, yn1, &context.panel);
                    let (x_pix2, y_pix2) = context.coord.transform(x_final_n, yn2, &context.panel);

                    // Main range line
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

                    // Caps (Horizontal/Vertical bars at the ends of the whisker)
                    let cap_len = mark_config.cap_length as Precision;
                    if !is_flipped {
                        // Standard: Vertical whisker -> Horizontal caps
                        for py in [y_pix1 as Precision, y_pix2 as Precision] {
                            let px = x_pix1 as Precision;
                            backend.draw_line(LineConfig {
                                x1: px - cap_len,
                                y1: py,
                                x2: px + cap_len,
                                y2: py,
                                color: mark_config.color,
                                width: mark_config.stroke_width as Precision,
                                opacity: mark_config.opacity as Precision,
                                dash: vec![],
                            });
                        }
                    } else {
                        // Flipped: Horizontal whisker -> Vertical caps
                        for px in [x_pix1 as Precision, x_pix2 as Precision] {
                            let py = y_pix1 as Precision;
                            backend.draw_line(LineConfig {
                                x1: px,
                                y1: py - cap_len,
                                x2: px,
                                y2: py + cap_len,
                                color: mark_config.color,
                                width: mark_config.stroke_width as Precision,
                                opacity: mark_config.opacity as Precision,
                                dash: vec![],
                            });
                        }
                    }
                }

                // 4.3 Draw Center Point (Optional circle representing Mean/Median)
                if mark_config.show_center {
                    if let Some(ycn) = yc_norms[idx] {
                        let (cx, cy) = context.coord.transform(x_final_n, ycn, &context.panel);

                        // Resolve color based on mapping or fallback
                        let group_color = if let Some(ref norms) = color_norms {
                            self.resolve_color_from_value(norms[idx], context, &mark_config.color)
                        } else {
                            mark_config.color
                        };

                        backend.draw_circle(CircleConfig {
                            x: cx as Precision,
                            y: cy as Precision,
                            radius: 3.0, // Fixed radius for center indicator
                            fill: group_color,
                            stroke: group_color,
                            stroke_width: 0.0,
                            opacity: mark_config.opacity as Precision,
                        });
                    }
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
