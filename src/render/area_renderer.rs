use crate::core::layer::{MarkRenderer, RenderBackend, PolygonConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::Precision;
use crate::mark::area::MarkArea;
use crate::error::ChartonError;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkArea> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        if df_source.df.height() == 0 { return Ok(()); }

        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkArea configuration is missing".to_string()))?;

        // 1. GROUPING
        let group_column = context.spec.aesthetics.color.as_ref().map(|c| c.field.as_str());
        let groups = match group_column {
            Some(col_name) => df_source.df.partition_by([col_name], true)?,
            None => vec![df_source.df.clone()],
        };

        let x_enc = self.encoding.x.as_ref().ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self.encoding.y.as_ref().ok_or(ChartonError::Encoding("Y missing".into()))?;

        for group_df in groups {
            // 2. AESTHETICS (Now calling the helper moved to Chart impl)
            let group_fill = self.resolve_group_color(&group_df, context, &mark_config.color)?;

            // 3. SORTING
            let sorted_df = group_df.sort(
                [x_enc.field.as_str()],
                SortMultipleOptions::default().with_order_descending(false)
            )?;

            let x_series = sorted_df.column(&x_enc.field)?.as_materialized_series();
            let y_series = sorted_df.column(&y_enc.field)?.as_materialized_series();

            let x_vals: Vec<f64> = x_series.f64()?.into_no_null_iter().collect();
            let y_vals: Vec<f64> = y_series.f64()?.into_no_null_iter().collect();

            if x_vals.is_empty() { continue; }

            // 4. PROJECTION
            let x_scale = context.coord.get_x_scale();
            let y_scale = context.coord.get_y_scale();
            let baseline_y_norm = y_scale.normalize(0.0);

            // Construct the closed polygon path
            let mut polygon_points: Vec<(Precision, Precision)> = Vec::with_capacity(x_vals.len() * 2);

            // Forward (Upper Boundary)
            for (&x, &y) in x_vals.iter().zip(y_vals.iter()) {
                let (px, py) = context.transform(x_scale.normalize(x), y_scale.normalize(y));
                polygon_points.push((px as Precision, py as Precision));
            }

            // Backward (Lower Boundary / Baseline)
            for &x in x_vals.iter().rev() {
                let (px, py_base) = context.transform(x_scale.normalize(x), baseline_y_norm);
                polygon_points.push((px as Precision, py_base as Precision));
            }

            // 5. DISPATCH (Fixed variable name to polygon_points)
            backend.draw_polygon(PolygonConfig {
                points: polygon_points,
                fill: group_fill,
                stroke: group_fill,
                stroke_width: mark_config.stroke_width as Precision,
                fill_opacity: mark_config.opacity as Precision,
                stroke_opacity: 1.0,
            });
        }

        Ok(())
    }
}