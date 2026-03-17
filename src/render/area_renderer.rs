use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{LineConfig, MarkRenderer, PathConfig, PolygonConfig, RenderBackend};
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

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // --- STEP 1: RENDER ZERO LINE (Baseline) ---
        // We draw the zero line first so it sits behind the data.
        // It is only drawn if 0.0 is within the visible Y-axis domain.
        let (y_min, y_max) = y_scale.domain();
        if y_min <= 0.0 && y_max >= 0.0 {
            let baseline_y_norm = y_scale.normalize(0.0);

            // Map the normalized 0.0 to physical coordinates across the full width/height.
            let (px1, py1) = context.transform(0.0, baseline_y_norm);
            let (px2, py2) = context.transform(1.0, baseline_y_norm);

            backend.draw_line(LineConfig {
                x1: px1 as Precision,
                y1: py1 as Precision,
                x2: px2 as Precision,
                y2: py2 as Precision,
                color: "#888888".into(), // Subtle gray for reference
                width: 1.0,
                opacity: 0.5,
                dash: vec![4.0, 4.0], // Dashed pattern: 4px dash, 4px gap
            });
        }

        // --- STEP 2: GROUPING ---
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

        for group_df in groups {
            // Determine the color for this specific group/category
            let group_base_color =
                self.resolve_group_color(&group_df, context, &mark_config.color)?;

            // 3. SORTING: Area charts must be sorted by X to prevent "zigzag" artifacts
            let sorted_df = group_df.sort(
                [x_enc.field.as_str()],
                SortMultipleOptions::default().with_order_descending(false),
            )?;

            let x_series = sorted_df.column(&x_enc.field)?.as_materialized_series();
            let y_series = sorted_df.column(&y_enc.field)?.as_materialized_series();

            let x_vals: Vec<f64> = x_series.f64()?.into_no_null_iter().collect();
            let y_vals: Vec<f64> = y_series.f64()?.into_no_null_iter().collect();

            if x_vals.is_empty() {
                continue;
            }

            // --- STEP 4: PROJECTION & POINT DECOUPLING ---
            let baseline_y_norm = y_scale.normalize(0.0);

            // fill_points: A closed loop including the baseline for the color fill.
            // stroke_points: An open sequence of points for the top boundary line.
            let mut fill_points: Vec<(Precision, Precision)> = Vec::with_capacity(x_vals.len() * 2);
            let mut stroke_points: Vec<(Precision, Precision)> = Vec::with_capacity(x_vals.len());

            // A: Construct Upper Boundary (The actual data points)
            for (&x, &y) in x_vals.iter().zip(y_vals.iter()) {
                let (px, py) = context.transform(x_scale.normalize(x), y_scale.normalize(y));
                let point = (px as Precision, py as Precision);

                stroke_points.push(point); // Add to the open path for the spine
                fill_points.push(point); // Add to the polygon for the fill area
            }

            // B: Construct Lower Boundary (Closing the polygon back along the baseline)
            for &x in x_vals.iter().rev() {
                let (px, py_base) = context.transform(x_scale.normalize(x), baseline_y_norm);
                fill_points.push((px as Precision, py_base as Precision));
            }

            // --- STEP 5: TWO-LAYER RENDERING ---

            // Layer 1: The Area Fill
            // We set stroke to None. This ensures that the bottom and sides of the area
            // do not have thick borders that clash with the axes or zero line.
            backend.draw_polygon(PolygonConfig {
                points: fill_points,
                fill: group_base_color,
                stroke: "none".into(), // No stroke on the polygon layer
                stroke_width: 0.0,
                fill_opacity: mark_config.opacity as Precision,
                stroke_opacity: 0.0,
            });

            // Layer 2: The Top Boundary Path
            // Using draw_path ensures we only stroke the "peaks" of the area chart.
            // This provides a sharp, professional look similar to ggplot2 or Altair.
            backend.draw_path(PathConfig {
                points: stroke_points,
                stroke: group_base_color,
                stroke_width: mark_config.stroke_width as Precision,
                opacity: 1.0, // Top line is opaque to stand out
                dash: mark_config.dash.iter().map(|&d| d as Precision).collect(),
            });
        }

        Ok(())
    }
}

impl Chart<MarkArea> {
    /// Resolves a single fill color for the entire area group.
    fn resolve_group_color(
        &self,
        df: &DataFrame,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();

            // Map the first value of the group to a color.
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
