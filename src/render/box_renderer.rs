use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{CircleConfig, LineConfig, MarkRenderer, RectConfig, RenderBackend};
use crate::error::ChartonError;
use crate::mark::boxplot::MarkBoxplot;
use crate::visual::color::SingleColor;
use crate::{Precision, TEMP_SUFFIX};

impl MarkRenderer for Chart<MarkBoxplot> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        if df_source.df.height() == 0 {
            return Ok(());
        }

        let mark_config = self.mark.as_ref().unwrap();
        let x_name = &self.encoding.x.as_ref().unwrap().field;

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // --- STEP 1: Vectorized Normalization ---
        // Map data values to [0, 1] normalized space for both axes.
        let x_norms = x_scale
            .scale_type()
            .normalize_series(x_scale, &df_source.column(x_name)?)?;
        let q1_n = y_scale
            .scale_type()
            .normalize_series(y_scale, &df_source.column(&format!("{}_q1", TEMP_SUFFIX))?)?;
        let q3_n = y_scale
            .scale_type()
            .normalize_series(y_scale, &df_source.column(&format!("{}_q3", TEMP_SUFFIX))?)?;
        let med_n = y_scale.scale_type().normalize_series(
            y_scale,
            &df_source.column(&format!("{}_median", TEMP_SUFFIX))?,
        )?;
        let min_n = y_scale
            .scale_type()
            .normalize_series(y_scale, &df_source.column(&format!("{}_min", TEMP_SUFFIX))?)?;
        let max_n = y_scale
            .scale_type()
            .normalize_series(y_scale, &df_source.column(&format!("{}_max", TEMP_SUFFIX))?)?;

        let outlier_series = df_source.column(&format!("{}_outliers", TEMP_SUFFIX))?;
        let outliers_col = outlier_series.list()?;

        // --- STEP 2: Color Mapping ---
        let color_vec: Vec<SingleColor> = if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            let norms = s_trait.scale_type().normalize_series(s_trait, &s)?;
            let l_max = s_trait.logical_max();

            norms
                .into_iter()
                .map(|opt_n| {
                    s_trait
                        .mapper()
                        .map(|m| m.map_to_color(opt_n.unwrap_or(0.0), l_max))
                        .unwrap_or_else(|| SingleColor::new("#333333"))
                })
                .collect()
        } else {
            vec![mark_config.color; df_source.df.height()]
        };

        let groups_count_col = df_source.column(&format!("{}_groups_count", TEMP_SUFFIX))?;
        let groups_count_col = groups_count_col.f64()?;
        let sub_idx_col = df_source.column(&format!("{}_sub_idx", TEMP_SUFFIX))?;
        let sub_idx_col = sub_idx_col.f64()?;

        // --- STEP 3: Calculate Unit Step in Normalized Space ---
        // This calculates how wide "1.0 data unit" is in the [0, 1] normalized range.
        // It accounts for padding/expansion in DiscreteScale.
        let n0 = x_scale.normalize(0.0);
        let n1 = x_scale.normalize(1.0);
        let unit_step_norm = (n1 - n0).abs();

        // --- STEP 4: Render Loop ---
        for i in 0..df_source.df.height() {
            let total_groups = groups_count_col.get(i).unwrap();
            let sub_idx = sub_idx_col.get(i).unwrap();

            // Apply dodge logic: calculate box width and spacing relative to unit_step_norm
            let box_width_data = mark_config.width.min(
                mark_config.span / (total_groups + (total_groups - 1.0) * mark_config.spacing),
            );
            let box_width_norm = box_width_data * unit_step_norm;
            let spacing_norm = box_width_norm * mark_config.spacing;

            // Offset the box center from the category tick position
            let offset_norm =
                (sub_idx - (total_groups - 1.0) / 2.0) * (box_width_norm + spacing_norm);
            let x_center_n = x_norms.get(i).unwrap() + offset_norm;

            let current_color = &color_vec[i];

            // Get normalized Y-stats
            let n_q1 = q1_n.get(i).unwrap();
            let n_q3 = q3_n.get(i).unwrap();
            let n_med = med_n.get(i).unwrap();
            let n_min = min_n.get(i).unwrap();
            let n_max = max_n.get(i).unwrap();

            // --- 5. Draw Rect (The Box) ---
            // Transform both corners. context.transform handles coord_flip automatically.
            let (bx1, by1) = context.transform(x_center_n - box_width_norm / 2.0, n_q1);
            let (bx2, by2) = context.transform(x_center_n + box_width_norm / 2.0, n_q3);

            backend.draw_rect(RectConfig {
                x: bx1.min(bx2) as Precision,
                y: by1.min(by2) as Precision,
                width: (bx1 - bx2).abs() as Precision,
                height: (by1 - by2).abs() as Precision,
                fill: *current_color,
                stroke: mark_config.stroke,
                stroke_width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
            });

            // --- 6. Draw Whiskers ---
            // We transform the full (x, y) pairs to avoid mixing axes manually.
            // This ensures whiskers orient correctly whether vertical or horizontal.
            let (p_min_x, p_min_y) = context.transform(x_center_n, n_min);
            let (p_max_x, p_max_y) = context.transform(x_center_n, n_max);
            let (p_q1_x, p_q1_y) = context.transform(x_center_n, n_q1);
            let (p_q3_x, p_q3_y) = context.transform(x_center_n, n_q3);

            // Lower whisker: Min to Q1
            backend.draw_line(LineConfig {
                x1: p_min_x as Precision,
                y1: p_min_y as Precision,
                x2: p_q1_x as Precision,
                y2: p_q1_y as Precision,
                color: mark_config.stroke,
                width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
                dash: None,
            });
            // Upper whisker: Max to Q3
            backend.draw_line(LineConfig {
                x1: p_max_x as Precision,
                y1: p_max_y as Precision,
                x2: p_q3_x as Precision,
                y2: p_q3_y as Precision,
                color: mark_config.stroke,
                width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
                dash: None,
            });

            // --- 7. Draw Median Line ---
            let (m1x, m1y) = context.transform(x_center_n - box_width_norm / 2.0, n_med);
            let (m2x, m2y) = context.transform(x_center_n + box_width_norm / 2.0, n_med);
            backend.draw_line(LineConfig {
                x1: m1x as Precision,
                y1: m1y as Precision,
                x2: m2x as Precision,
                y2: m2y as Precision,
                color: mark_config.stroke,
                width: (mark_config.stroke_width * 2.0) as Precision,
                opacity: mark_config.opacity as Precision,
                dash: None,
            });

            // --- 8. Draw Outliers ---
            if let Some(s_outliers) = outliers_col.get_as_series(i)
                && !s_outliers.is_empty()
            {
                let n_outliers = y_scale
                    .scale_type()
                    .normalize_series(y_scale, &s_outliers)?;
                for n_o_opt in n_outliers.into_iter() {
                    if let Some(n_o) = n_o_opt {
                        // Outliers also need full (x, y) transform to follow the flip.
                        let (ox, oy) = context.transform(x_center_n, n_o);
                        backend.draw_circle(CircleConfig {
                            x: ox as Precision,
                            y: oy as Precision,
                            radius: mark_config.outlier_size as Precision,
                            fill: mark_config.outlier_color,
                            stroke: SingleColor::new("none"),
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
