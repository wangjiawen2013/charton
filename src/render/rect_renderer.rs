use crate::core::layer::{MarkRenderer, RenderBackend, RectConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::mark::rect::MarkRect;
use crate::error::ChartonError;
use crate::Precision;
use crate::visual::color::SingleColor;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkRect> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df = &self.data.df;
        if df.height() == 0 { return Ok(()); }

        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkRect configuration is missing".into()))?;

        // --- STEP 1: POSITIONING ---
        let x_enc = self.encoding.x.as_ref().ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self.encoding.y.as_ref().ok_or(ChartonError::Encoding("Y missing".into()))?;
        
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        let x_series = df.column(&x_enc.field)?.as_materialized_series();
        let y_series = df.column(&y_enc.field)?.as_materialized_series();

        let x_norms = x_scale.scale_type().normalize_series(x_scale, x_series)?;
        let y_norms = y_scale.scale_type().normalize_series(y_scale, y_series)?;

        // --- STEP 2: SIZE CALCULATION ---
        // 使用 Float64Chunked 的唯一值数量来计算步长
        let (rect_width, rect_height) = self.calculate_rect_size(context, &x_norms, &y_norms);

        // --- STEP 3: COLOR MAPPING ---
        let color_iter = self.resolve_rect_colors(df, context, &mark_config.color)?;

        // --- STEP 4: RENDERING LOOP ---
        // 使用 Polars 迭代器高效遍历归一化后的数据
        for ((opt_x, opt_y), fill_color) in x_norms.into_iter()
            .zip(y_norms.into_iter())
            .zip(color_iter) 
        {
            let x_n = opt_x.unwrap_or(0.0);
            let y_n = opt_y.unwrap_or(0.0);

            let (px, py) = context.transform(x_n, y_n);

            backend.draw_rect(RectConfig {
                x: (px - rect_width / 2.0) as Precision,
                y: (py - rect_height / 2.0) as Precision,
                width: rect_width as Precision,
                height: rect_height as Precision,
                fill: fill_color,
                stroke: mark_config.stroke.clone(),
                stroke_width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
            });
        }

        Ok(())
    }
}

impl Chart<MarkRect> {
    /// 计算矩形尺寸：基于归一化空间中唯一值的密度
    fn calculate_rect_size(&self, context: &PanelContext, x_ca: &Float64Chunked, y_ca: &Float64Chunked) -> (f64, f64) {
        // Polars 的 n_unique() 非常快
        let x_count = x_ca.n_unique().unwrap_or(1);
        let y_count = y_ca.n_unique().unwrap_or(1);

        let x_step = if x_count > 1 { 1.0 / (x_count as f64 - 1.0) } else { 0.1 };
        let y_step = if y_count > 1 { 1.0 / (y_count as f64 - 1.0) } else { 0.1 };

        let (p0_x, p0_y) = context.transform(0.0, 0.0);
        let (p1_x, p1_y) = context.transform(x_step, y_step);

        ((p1_x - p0_x).abs(), (p1_y - p0_y).abs())
    }

    /// 解析颜色流：支持数据驱动映射或静态回退
    fn resolve_rect_colors(
        &self, 
        df: &DataFrame, 
        context: &PanelContext, 
        fallback: &SingleColor
    ) -> Result<Box<dyn Iterator<Item = SingleColor>>, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            
            // 使用新的 normalize_series 接口
            let norms = s_trait.scale_type().normalize_series(s_trait, s)?;
            let l_max = s_trait.logical_max();

            let colors: Vec<SingleColor> = norms.into_iter()
                .map(|opt_n| {
                    s_trait.mapper()
                        .map(|m| m.map_to_color(opt_n.unwrap_or(0.0), l_max))
                        .unwrap_or_else(|| fallback.clone())
                })
                .collect();
            Ok(Box::new(colors.into_iter()))
        } else {
            Ok(Box::new(std::iter::repeat(fallback.clone())))
        }
    }
}