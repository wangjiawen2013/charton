use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{CircleConfig, LineConfig, MarkRenderer, RectConfig, RenderBackend};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::boxplot::MarkBoxplot;
use crate::visual::color::SingleColor;
use crate::{Precision, TEMP_SUFFIX};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl MarkRenderer for Chart<MarkBoxplot> {
    /// Renders Boxplots using high-performance parallel geometry generation.
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

        // --- STEP 1: INITIALIZATION & VALIDATION ---
        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("Boxplot config missing".into()))?;
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X encoding missing".into()))?;

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // --- STEP 2: DATA NORMALIZATION ---
        // Normalize primary axes and all statistical summary columns
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, df_source.column(&x_enc.field)?);
        let q1_n = y_scale
            .scale_type()
            .normalize_column(y_scale, df_source.column(&format!("{}_q1", TEMP_SUFFIX))?);
        let q3_n = y_scale
            .scale_type()
            .normalize_column(y_scale, df_source.column(&format!("{}_q3", TEMP_SUFFIX))?);
        let med_n = y_scale.scale_type().normalize_column(
            y_scale,
            df_source.column(&format!("{}_median", TEMP_SUFFIX))?,
        );
        let min_n = y_scale
            .scale_type()
            .normalize_column(y_scale, df_source.column(&format!("{}_min", TEMP_SUFFIX))?);
        let max_n = y_scale
            .scale_type()
            .normalize_column(y_scale, df_source.column(&format!("{}_max", TEMP_SUFFIX))?);

        // Normalize color aesthetic if a mapping exists
        let color_norms = context.spec.aesthetics.color.as_ref().map(|m| {
            let s = m.scale_impl.as_ref();
            s.scale_type()
                .normalize_column(s, df_source.column(&m.field).unwrap())
        });

        // Retrieve helper columns for positioning and outliers
        let groups_count_col = df_source.column(&format!("{}_groups_count", TEMP_SUFFIX))?;
        let sub_idx_col = df_source.column(&format!("{}_sub_idx", TEMP_SUFFIX))?;
        let outliers_col = df_source.column(&format!("{}_outliers", TEMP_SUFFIX))?;

        // Constant used for calculating horizontal pixel widths relative to the X-axis scale
        let unit_step_norm = (x_scale.normalize(1.0) - x_scale.normalize(0.0)).abs();

        // --- STEP 3: PARALLEL GEOMETRY COMPUTATION ---
        let x_col = df_source.column(&x_enc.field)?;
        let boundary_tag = format!("{}_boundary", TEMP_SUFFIX);

        let box_elements: Vec<BoxElement> = (0..row_count)
            .maybe_into_par_iter()
            .filter_map(|i| {
                // 1. Ghost/Boundary Check
                let x_val = x_col.get_str(i)?;
                if x_val == boundary_tag {
                    return None;
                }

                // 2. Comprehensive Data Validation (Filter out Gaps)
                // Use the '?' operator to skip rows where any essential stat is NaN/None
                let q1_val = q1_n[i]?;
                let q3_val = q3_n[i]?;
                let med_val = med_n[i]?;
                let min_val = min_n[i]?;
                let max_val = max_n[i]?;

                let total_groups = groups_count_col.get_f64(i).unwrap_or(1.0);
                let sub_idx = sub_idx_col.get_f64(i).unwrap_or(0.0);

                // --- DODGE LOGIC ---
                let box_width_data = mark_config.width.min(
                    mark_config.span / (total_groups + (total_groups - 1.0) * mark_config.spacing),
                );
                let box_width_norm = box_width_data * unit_step_norm;
                let spacing_norm = box_width_norm * mark_config.spacing;
                let offset_norm =
                    (sub_idx - (total_groups - 1.0) / 2.0) * (box_width_norm + spacing_norm);

                let x_center_n = x_norms[i]? + offset_norm;

                // --- RESOLVE COLOR ---
                let fill = if let Some(ref norms) = color_norms {
                    self.resolve_color_from_value(norms[i], context, &mark_config.color)
                } else {
                    mark_config.color
                };

                // --- COORDINATE PROJECTION ---
                // Reuse the local validated variables (q1_val, med_val, etc.)
                // to avoid redundant indexing and Option unwrapping.
                let (bx1, by1) = context.coord.transform(
                    x_center_n - box_width_norm / 2.0,
                    q1_val,
                    &context.panel,
                );
                let (bx2, by2) = context.coord.transform(
                    x_center_n + box_width_norm / 2.0,
                    q3_val,
                    &context.panel,
                );

                let (p_min_x, p_min_y) =
                    context.coord.transform(x_center_n, min_val, &context.panel);
                let (p_max_x, p_max_y) =
                    context.coord.transform(x_center_n, max_val, &context.panel);

                let (p_q1_x, p_q1_y) = context.coord.transform(x_center_n, q1_val, &context.panel);
                let (p_q3_x, p_q3_y) = context.coord.transform(x_center_n, q3_val, &context.panel);

                let (m1x, m1y) = context.coord.transform(
                    x_center_n - box_width_norm / 2.0,
                    med_val,
                    &context.panel,
                );
                let (m2x, m2y) = context.coord.transform(
                    x_center_n + box_width_norm / 2.0,
                    med_val,
                    &context.panel,
                );

                // --- OUTLIER PARSING ---
                let mut outlier_circles = Vec::new();
                if mark_config.show_outliers
                    && let Some(raw_outliers) = outliers_col.get_str(i)
                {
                    let clean = raw_outliers.trim_matches(|c| c == '[' || c == ']');
                    for val_str in clean.split(',').filter(|s| !s.trim().is_empty()) {
                        if let Ok(val) = val_str.trim().parse::<f64>() {
                            let n_o = y_scale.normalize(val);
                            let (ox, oy) = context.coord.transform(x_center_n, n_o, &context.panel);
                            outlier_circles.push(CircleConfig {
                                x: ox as Precision,
                                y: oy as Precision,
                                radius: mark_config.outlier_size as Precision,
                                fill: mark_config.outlier_color,
                                stroke: SingleColor::new("none"),
                                stroke_width: 0.0,
                                opacity: mark_config.opacity as Precision, // fill opacity
                            });
                        }
                    }
                }

                Some(BoxElement {
                    rect: RectConfig {
                        x: bx1.min(bx2) as Precision,
                        y: by1.min(by2) as Precision,
                        width: (bx1 - bx2).abs() as Precision,
                        height: (by1 - by2).abs() as Precision,
                        fill,
                        stroke: mark_config.stroke,
                        stroke_width: mark_config.stroke_width as Precision,
                        opacity: mark_config.opacity as Precision, // fill opacity
                    },
                    whisker_low: [p_min_x, p_min_y, p_q1_x, p_q1_y],
                    whisker_high: [p_max_x, p_max_y, p_q3_x, p_q3_y],
                    median_line: [m1x, m1y, m2x, m2y],
                    outliers: outlier_circles,
                })
            })
            .collect();

        // --- STEP 4: SEQUENTIAL RENDERING ---
        // Backend draw calls are executed on the main thread.
        for el in box_elements {
            // Draw main box
            backend.draw_rect(el.rect);

            // Draw whiskers
            backend.draw_line(LineConfig {
                x1: el.whisker_low[0] as Precision,
                y1: el.whisker_low[1] as Precision,
                x2: el.whisker_low[2] as Precision,
                y2: el.whisker_low[3] as Precision,
                color: mark_config.stroke,
                width: mark_config.stroke_width as Precision,
                opacity: 1.0,
                dash: vec![],
            });
            backend.draw_line(LineConfig {
                x1: el.whisker_high[0] as Precision,
                y1: el.whisker_high[1] as Precision,
                x2: el.whisker_high[2] as Precision,
                y2: el.whisker_high[3] as Precision,
                color: mark_config.stroke,
                width: mark_config.stroke_width as Precision,
                opacity: 1.0,
                dash: vec![],
            });

            // Draw median line (bold)
            backend.draw_line(LineConfig {
                x1: el.median_line[0] as Precision,
                y1: el.median_line[1] as Precision,
                x2: el.median_line[2] as Precision,
                y2: el.median_line[3] as Precision,
                color: mark_config.stroke,
                width: (mark_config.stroke_width * 2.0) as Precision,
                opacity: 1.0,
                dash: vec![],
            });

            // Draw outliers
            for outlier in el.outliers {
                backend.draw_circle(outlier);
            }
        }

        Ok(())
    }
}

/// Stores pre-calculated screen coordinates for a single boxplot group.
struct BoxElement {
    rect: RectConfig,
    whisker_low: [f64; 4],
    whisker_high: [f64; 4],
    median_line: [f64; 4],
    outliers: Vec<CircleConfig>,
}

impl Chart<MarkBoxplot> {
    /// Resolves aesthetic color based on normalized values and scale mapping.
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
