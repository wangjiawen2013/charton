use crate::core::layer::{MarkRenderer, RenderBackend, PolygonConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::mark::bar::MarkBar;
use crate::visual::color::SingleColor;
use crate::error::ChartonError;
use crate::Precision;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkBar> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df = &self.data.df;
        if df.height() == 0 { return Ok(()); }

        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkBar config missing".into()))?;

        // --- STEP 1: Encoding & Scales ---
        let x_enc = self.encoding.x.as_ref().ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self.encoding.y.as_ref().ok_or(ChartonError::Encoding("Y missing".into()))?;
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        let is_stacked = y_enc.stack;
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());

        // --- STEP 2: Calculate Unit Step in Normalized Space ---
        // 计算归一化空间中“一个类目单位”的大小，通常受 Scale Padding 影响
        let n0 = x_scale.normalize(0.0);
        let n1 = x_scale.normalize(1.0);
        let unit_step_norm = (n1 - n0).abs();

        // --- STEP 3: Handle Grouping & Dodge Meta ---
        // 我们需要知道每个 X 坐标处有多少个子组 (Dodge count)
        // 假设 transform_bar_data 已经保证了数据的对齐，
        // 我们通过颜色分列来识别组信息
        let groups = match color_field {
            Some(col) => df.partition_by_stable([col], true)?,
            None => vec![df.clone()],
        };
        let n_groups = groups.len() as f64;

        // 计算单个子条形在数据空间的宽度
        // 逻辑：如果堆叠，则宽度由 span 决定；如果并列，则由 span / 分组数（含间距）决定
        let bar_width_data = if is_stacked || n_groups <= 1.0 {
            mark_config.width.min(mark_config.span)
        } else {
            mark_config.width.min(
                mark_config.span / (n_groups + (n_groups - 1.0) * mark_config.spacing)
            )
        };

        let bar_width_norm = bar_width_data * unit_step_norm;
        let spacing_norm = bar_width_norm * mark_config.spacing;

        // 用于 Stacked 模式的累加器
        let mut stack_acc = Vec::new();

        // --- STEP 4: Render Loop ---
        for (group_idx, group_df) in groups.iter().enumerate() {
            let group_color = self.resolve_group_color(group_df, context, &mark_config.color)?;
            
            let x_series = group_df.column(&x_enc.field)?.as_materialized_series();
            let y_series = group_df.column(&y_enc.field)?.as_materialized_series();

            let x_norms = x_scale.scale_type().normalize_series(x_scale, x_series)?;
            let y_vals: Vec<f64> = y_series.f64()?.into_no_null_iter().collect();

            for (i, (opt_x_n, y_val)) in x_norms.into_iter().zip(y_vals).enumerate() {
                let x_tick_n = opt_x_n.unwrap_or(0.0);

                // Y 轴起止 (归一化)
                let (y_low_n, y_high_n) = if is_stacked {
                    if stack_acc.len() <= i { stack_acc.push(0.0); }
                    let start = stack_acc[i];
                    let end = start + y_val;
                    stack_acc[i] = end;
                    (y_scale.normalize(start), y_scale.normalize(end))
                } else {
                    (y_scale.normalize(0.0), y_scale.normalize(y_val))
                };

                // --- Dodge Offset Calculation ---
                // 计算该子组中心相对于类目中心 (Tick) 的偏移
                let offset_norm = if !is_stacked && n_groups > 1.0 {
                    (group_idx as f64 - (n_groups - 1.0) / 2.0) * (bar_width_norm + spacing_norm)
                } else {
                    0.0
                };

                let x_center_n = x_tick_n + offset_norm;
                let left_n = x_center_n - bar_width_norm / 2.0;
                let right_n = x_center_n + bar_width_norm / 2.0;

                // --- Transform to Pixels ---
                let p1 = context.transform(left_n, y_low_n);
                let p2 = context.transform(left_n, y_high_n);
                let p3 = context.transform(right_n, y_high_n);
                let p4 = context.transform(right_n, y_low_n);

                backend.draw_polygon(PolygonConfig {
                    points: vec![
                        (p1.0 as Precision, p1.1 as Precision),
                        (p2.0 as Precision, p2.1 as Precision),
                        (p3.0 as Precision, p3.1 as Precision),
                        (p4.0 as Precision, p4.1 as Precision),
                    ],
                    fill: group_color.clone(),
                    stroke: mark_config.stroke.clone(),
                    stroke_width: mark_config.stroke_width as Precision,
                    fill_opacity: mark_config.opacity as Precision,
                    stroke_opacity: mark_config.opacity as Precision,
                });
            }
        }

        Ok(())
    }
}

impl Chart<MarkBar> {
    fn resolve_group_color(&self, df: &DataFrame, context: &PanelContext, fallback: &SingleColor) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            let norms = s_trait.scale_type().normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = norms.get(0).unwrap_or(0.0);
            
            Ok(s_trait.mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| fallback.clone()))
        } else {
            Ok(fallback.clone())
        }
    }
}