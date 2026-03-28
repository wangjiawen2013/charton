use crate::Precision;
use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{LineConfig, MarkRenderer, PathConfig, PolygonConfig, RenderBackend};
use crate::encode::y::StackMode;
use crate::error::ChartonError;
use crate::mark::area::MarkArea;
use crate::visual::color::SingleColor;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkArea> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        if df_source.df.height() == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkArea configuration is missing".to_string()))?;

        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or(ChartonError::Encoding("Y missing".into()))?;

        let y_field = y_enc.field.as_str();
        let y0 = format!("{}_{}_min", TEMP_SUFFIX, y_field);
        let y1 = format!("{}_{}_max", TEMP_SUFFIX, y_field);

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // ========================================================================
        // STEP 1: RENDER ZERO LINE (Baseline) - Only for StackMode::None
        // ========================================================================
        let (y_min, y_max) = y_scale.domain();
        if y_min <= 0.0 && y_max >= 0.0 {
            let baseline_y_norm = y_scale.normalize(0.0);
            let (px1, py1) = context.transform(0.0, baseline_y_norm);
            let (px2, py2) = context.transform(1.0, baseline_y_norm);

            backend.draw_line(LineConfig {
                x1: px1 as Precision,
                y1: py1 as Precision,
                x2: px2 as Precision,
                y2: py2 as Precision,
                color: "#888888".into(),
                width: 1.0,
                opacity: 0.5,
                dash: vec![4.0, 4.0],
            });
        }

        // ========================================================================
        // STEP 2: GROUPING BY COLOR
        // ========================================================================
        let group_column = context
            .spec
            .aesthetics
            .color
            .as_ref()
            .map(|c| c.field.as_str());
        let groups = match group_column {
            Some(col_name) => df_source.df.partition_by([col_name], true)?,
            None => vec![df_source.df.clone()],
        };

        // Determine if we should use y0/y1 columns (stacked modes) or raw y values
        let use_stacked_columns = matches!(
            y_enc.stack,
            StackMode::Stacked | StackMode::Normalize | StackMode::Center
        );

        for group_df in groups {
            let group_base_color =
                self.resolve_group_color(&group_df, context, &mark_config.color)?;

            // ========================================================================
            // STEP 3: SORTING BY X
            // ========================================================================
            let sorted_df = group_df.sort(
                [x_enc.field.as_str()],
                SortMultipleOptions::default().with_order_descending(false),
            )?;

            // Handle both numeric and temporal types for the X-axis
            // Timestamps (i64) must be cast to f64 for coordinate projection.
            let x_series = sorted_df.column(&x_enc.field)?.as_materialized_series();
            let x_vals: Vec<f64> = if x_series.dtype().is_temporal() {
                x_series
                    .cast(&DataType::Int64)?
                    .i64()?
                    .into_no_null_iter()
                    .map(|v| v as f64)
                    .collect()
            } else {
                x_series.f64()?.into_no_null_iter().collect()
            };

            if x_vals.is_empty() {
                continue;
            }

            // ========================================================================
            // STEP 4: EXTRACT Y VALUES (y0/y1 for stacked, raw y for none)
            // ========================================================================
            let y0_vals: Vec<f64>;
            let y1_vals: Vec<f64>;

            if use_stacked_columns {
                // Use pre-calculated y0 (baseline) and y1 (top) from transform
                let y0_series = sorted_df.column(&y0)?.as_materialized_series();
                let y1_series = sorted_df.column(&y1)?.as_materialized_series();
                y0_vals = y0_series.f64()?.into_no_null_iter().collect();
                y1_vals = y1_series.f64()?.into_no_null_iter().collect();
            } else {
                // Use raw y values with 0.0 baseline (StackMode::None)
                let y_series = sorted_df.column(&y_enc.field)?.as_materialized_series();
                let y_vals: Vec<f64> = y_series.f64()?.into_no_null_iter().collect();
                y0_vals = vec![0.0; y_vals.len()];
                y1_vals = y_vals;
            }

            // ========================================================================
            // STEP 5: PROJECTION & POINT DECOUPLING
            // ========================================================================
            let mut fill_points: Vec<(Precision, Precision)> = Vec::with_capacity(x_vals.len() * 2);
            let mut stroke_points: Vec<(Precision, Precision)> = Vec::with_capacity(x_vals.len());

            // A: Construct Upper Boundary (y1 values)
            for (&x, &y1) in x_vals.iter().zip(y1_vals.iter()) {
                let (px, py) = context.transform(x_scale.normalize(x), y_scale.normalize(y1));
                let point = (px as Precision, py as Precision);
                stroke_points.push(point);
                fill_points.push(point);
            }

            // B: Construct Lower Boundary (y0 values, reversed)
            for (&x, &y0) in x_vals.iter().rev().zip(y0_vals.iter().rev()) {
                let (px, py_base) = context.transform(x_scale.normalize(x), y_scale.normalize(y0));
                fill_points.push((px as Precision, py_base as Precision));
            }

            // ========================================================================
            // STEP 6: TWO-LAYER RENDERING
            // ========================================================================

            // Layer 1: The Area Fill
            backend.draw_polygon(PolygonConfig {
                points: fill_points,
                fill: group_base_color,
                stroke: "none".into(),
                stroke_width: 0.0,
                fill_opacity: mark_config.opacity as Precision,
                stroke_opacity: 0.0,
            });

            // Layer 2: The Top Boundary Path (Only for unstacked areas)
            // Stacked modes (Stacked, Normalize, Center) don't draw strokes to avoid
            // visual clutter and edge ambiguity issues in streamgraph visualization.
            if matches!(y_enc.stack, StackMode::None) {
                backend.draw_path(PathConfig {
                    points: stroke_points,
                    stroke: group_base_color,
                    stroke_width: mark_config.stroke_width as Precision,
                    opacity: 1.0,
                    dash: mark_config.dash.iter().map(|&d| d as Precision).collect(),
                });
            }
        }

        Ok(())
    }
}

impl Chart<MarkArea> {
    fn resolve_group_color(
        &self,
        df: &DataFrame,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();

            let first_val_norm = s_trait
                .scale_type()
                .normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = first_val_norm.get(0).unwrap_or(0.0);

            Ok(s_trait
                .mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| *fallback))
        } else {
            Ok(*fallback)
        }
    }
}
