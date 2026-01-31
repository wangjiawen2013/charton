use crate::core::layer::{MarkRenderer, RenderBackend, LineConfig, RectConfig, CircleConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::mark::boxplot::MarkBoxplot;
use crate::error::ChartonError;
use crate::visual::color::SingleColor;
use crate::Precision;

impl MarkRenderer for Chart<MarkBoxplot> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        if df_source.df.height() == 0 { return Ok(()); }

        let mark_config = self.mark.as_ref().unwrap();
        let x_name = &self.encoding.x.as_ref().unwrap().field;

        // --- STEP 1: POSITION NORMALIZATION ---
        // Get scale traits from the coordinate system
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // Vectorized normalization of all statistical columns
        let x_norms = x_scale.scale_type().normalize_series(x_scale, &df_source.column(x_name)?)?;
        let q1_n = y_scale.scale_type().normalize_series(y_scale, &df_source.column("q1")?)?;
        let q3_n = y_scale.scale_type().normalize_series(y_scale, &df_source.column("q3")?)?;
        let med_n = y_scale.scale_type().normalize_series(y_scale, &df_source.column("median")?)?;
        let min_n = y_scale.scale_type().normalize_series(y_scale, &df_source.column("min")?)?;
        let max_n = y_scale.scale_type().normalize_series(y_scale, &df_source.column("max")?)?;
        
        // Outliers are stored as a List column
        let outliers_col = df_source.column("outliers")?.list()?;

        // --- STEP 2: COLOR MAPPING (Aligned with PointChart) ---
        // Resolve data-driven color scale or fallback to a static mark color.
        let color_vec: Vec<SingleColor> = if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            
            // Normalize the aesthetic series using the mapper's specific scale logic.
            let norms = s_trait.scale_type().normalize_series(s_trait, &s)?;
            let l_max = s_trait.logical_max();
            
            norms.into_iter()
                .map(|opt_n| {
                    s_trait.mapper()
                        .map(|m| m.map_to_color(opt_n.unwrap_or(0.0), l_max))
                        .unwrap_or_else(|| SingleColor::from("#333333"))
                })
                .collect()
        } else {
            vec![mark_config.color.clone(); df_source.df.height()]
        };

        // --- STEP 3: RENDERING LOOP ---
        // Retrieve pre-calculated dodge parameters from transform_box_data
        let groups_count_col = df_source.column("groups_count")?.f64()?;
        let sub_idx_col = df_source.column("sub_idx")?.f64()?;

        for i in 0..df_source.df.height() {
            let total_groups = groups_count_col.get(i).unwrap();
            let sub_idx = sub_idx_col.get(i).unwrap();

            // Calculate the horizontal dodge offset in normalized space
            let box_width_norm = mark_config.width.min(
                mark_config.span / (total_groups + (total_groups - 1.0) * mark_config.spacing)
            );
            let spacing_norm = box_width_norm * mark_config.spacing;
            let offset_norm = (sub_idx - (total_groups - 1.0) / 2.0) * (box_width_norm + spacing_norm);
            
            // Final normalized X center
            let x_center_n = x_norms.get(i).unwrap() + offset_norm;
            let current_color = &color_vec[i];

            // Project statistical points to pixels
            let n_q1 = q1_n.get(i).unwrap();
            let n_q3 = q3_n.get(i).unwrap();
            let n_med = med_n.get(i).unwrap();
            let n_min = min_n.get(i).unwrap();
            let n_max = max_n.get(i).unwrap();

            // 1. Draw Rect (The Box / IQR)
            let (bx1, by1) = context.transform(x_center_n - box_width_norm / 2.0, n_q1);
            let (bx2, by2) = context.transform(x_center_n + box_width_norm / 2.0, n_q3);
            
            backend.draw_rect(RectConfig {
                x: bx1.min(bx2) as Precision,
                y: by1.min(by2) as Precision,
                width: (bx1 - bx2).abs() as Precision,
                height: (by1 - by2).abs() as Precision,
                fill: current_color.clone(),
                stroke: mark_config.stroke.clone(),
                stroke_width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
            });

            // 2. Draw Whiskers (The Lines)
            let (cx, py_min) = context.transform(x_center_n, n_min);
            let (_, py_max) = context.transform(x_center_n, n_max);
            let (_, py_q1) = context.transform(x_center_n, n_q1);
            let (_, py_q3) = context.transform(x_center_n, n_q3);

            let whisker_style = LineConfig {
                color: mark_config.stroke.clone(),
                width: mark_config.stroke_width as Precision,
                ..Default::default()
            };

            backend.draw_line(LineConfig { x1: cx as Precision, y1: py_min as Precision, x2: cx as Precision, y2: py_q1 as Precision, ..whisker_style.clone() });
            backend.draw_line(LineConfig { x1: cx as Precision, y1: py_max as Precision, x2: cx as Precision, y2: py_q3 as Precision, ..whisker_style.clone() });

            // 3. Draw Median (Thicker horizontal line)
            let (m1x, m1y) = context.transform(x_center_n - box_width_norm / 2.0, n_med);
            let (m2x, m2y) = context.transform(x_center_n + box_width_norm / 2.0, n_med);
            backend.draw_line(LineConfig {
                x1: m1x as Precision, y1: m1y as Precision,
                x2: m2x as Precision, y2: m2y as Precision,
                color: mark_config.stroke.clone(),
                width: mark_config.stroke_width as Precision * 2.0,
            });

            // 4. Draw Outliers (Individual points)
            let s_outliers = outliers_col.get_as_series(i).unwrap();
            if s_outliers.len() > 0 {
                let outliers_f64 = s_outliers.f64()?;
                for o_val in outliers_f64.into_iter().flatten() {
                    let n_o = y_scale.scale_type().normalize(y_scale, o_val);
                    let (ox, oy) = context.transform(x_center_n, n_o);
                    backend.draw_circle(CircleConfig {
                        cx: ox as Precision,
                        cy: oy as Precision,
                        r: mark_config.outlier_size as Precision,
                        fill: Some(mark_config.outlier_color.clone()),
                        stroke: None,
                        stroke_width: 0.0,
                        opacity: mark_config.opacity as Precision,
                    });
                }
            }
        }
        Ok(())
    }
}