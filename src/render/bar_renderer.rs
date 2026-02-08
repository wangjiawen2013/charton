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
        // Access scales and encodings. Note that x_scale and y_scale are retrieved
        // from the coordinate system, which has already been "trained" on the data.
        let x_enc = self.encoding.x.as_ref().ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self.encoding.y.as_ref().ok_or(ChartonError::Encoding("Y missing".into()))?;
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        let is_stacked = y_enc.stack;
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());

        // --- STEP 2: Calculate Unit Step in Normalized Space ---
        // We determine how wide "one unit" of the X-axis is in [0, 1] space.
        // This accounts for scale padding and range constraints.
        let n0 = x_scale.normalize(0.0);
        let n1 = x_scale.normalize(1.0);
        let unit_step_norm = (n1 - n0).abs();

        // --- STEP 3: Handle Grouping & Dodge Meta ---
        // Partition data by color for grouped (dodged) bar charts.
        let groups = match color_field {
            Some(col) => df.partition_by_stable([col], true)?,
            None => vec![df.clone()],
        };
        let n_groups = groups.len() as f64;

        // Calculate the logical width of a single bar.
        // If grouped (dodged), the bar width is divided by the number of groups plus spacing.
        let bar_width_data = if is_stacked || n_groups <= 1.0 {
            mark_config.width.min(mark_config.span)
        } else {
            mark_config.width.min(
                mark_config.span / (n_groups + (n_groups - 1.0) * mark_config.spacing)
            )
        };

        let bar_width_norm = bar_width_data * unit_step_norm;
        let spacing_norm = bar_width_norm * mark_config.spacing;

        // Accumulator for stacking Y values.
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

                // Determine vertical bounds in normalized [0, 1] space.
                let (y_low_n, y_high_n) = if is_stacked {
                    if stack_acc.len() <= i { stack_acc.push(0.0); }
                    let start = stack_acc[i];
                    let end = start + y_val;
                    stack_acc[i] = end;
                    (y_scale.normalize(start), y_scale.normalize(end))
                } else {
                    (y_scale.normalize(0.0), y_scale.normalize(y_val))
                };

                // Calculate horizontal dodge offset for grouped bars.
                let offset_norm = if !is_stacked && n_groups > 1.0 {
                    (group_idx as f64 - (n_groups - 1.0) / 2.0) * (bar_width_norm + spacing_norm)
                } else {
                    0.0
                };

                let x_center_n = x_tick_n + offset_norm;
                let left_n = x_center_n - bar_width_norm / 2.0;
                let right_n = x_center_n + bar_width_norm / 2.0;

                // --- THE MAGIC: Path-based Transformation ---
                // We define the bar as a polygon path in Normalized Space.
                // In a Polar system, the 'top' and 'bottom' horizontal segments 
                // must be curved to follow the radius.
                let rect_path = vec![
                    (left_n, y_low_n),   // Bottom-Left
                    (left_n, y_high_n),  // Top-Left
                    (right_n, y_high_n), // Top-Right
                    (right_n, y_low_n),  // Bottom-Right
                ];

                // Instead of calling transform() 4 times, we call transform_path().
                // This allows the coordinate system to perform "Adaptive Interpolation".
                // In Cartesian: Returns 4 points (straight lines).
                // In Polar: Returns ~20-50 points (curved arcs for the top/bottom).
                let pixel_points = context.transform_path(&rect_path, true);

                // Render the resulting points as a single polygon.
                backend.draw_polygon(PolygonConfig {
                    points: pixel_points.into_iter()
                        .map(|(x, y)| (x as Precision, y as Precision))
                        .collect(),
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
    /// Resolves the color for a specific data group based on global aesthetic mappings.
    fn resolve_group_color(
        &self, 
        df: &DataFrame, 
        context: &PanelContext, 
        fallback: &SingleColor
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            // Extract the first value of the group to determine the color.
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            
            // Normalize the data value to [0, 1] using the shared scale.
            let norms = s_trait.scale_type().normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = norms.get(0).unwrap_or(0.0);
            
            // Map the normalized value to a physical color using the scale's palette.
            Ok(s_trait.mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| fallback.clone()))
        } else {
            // No color mapping defined; use the default mark color.
            Ok(fallback.clone())
        }
    }
}