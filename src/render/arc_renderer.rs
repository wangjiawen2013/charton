use crate::core::layer::{MarkRenderer, RenderBackend, PolygonConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::mark::arc::MarkArc;
use crate::visual::color::SingleColor;
use crate::error::ChartonError;
use crate::Precision;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkArc> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df = &self.data.df;
        if df.height() == 0 { return Ok(()); }

        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkArc configuration missing".into()))?;

        // --- STEP 1: DIMENSION MAPPING ---
        // Data Y (Value) -> Angle (Theta)
        // Data X (Category) -> Radius (r)
        let y_enc = self.encoding.y.as_ref()
            .ok_or(ChartonError::Encoding("Theta (Y) encoding missing".into()))?;
        let is_stacked = y_enc.stack;

        // --- STEP 2: CALCULATE TOTAL SUM FOR NORMALIZATION ---
        // We need the total sum of all Y values to map them to [0, 1] (0 to 2π)
        let total_sum: f64 = df.column(&y_enc.field)?
            .f64()?
            .sum()
            .unwrap_or(0.0);

        if total_sum <= 0.0 && is_stacked {
            return Ok(()); // Avoid division by zero
        }

        // --- STEP 3: GROUPING ---
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());
        let groups = match color_field {
            Some(col) => df.partition_by_stable([col], true)?,
            None => vec![df.clone()],
        };

        // Normalized padding angle
        let pad_angle_norm = mark_config.pad_angle / (2.0 * std::f64::consts::PI);

        // --- STEP 4: RENDER LOOP ---
        let mut global_stack_pos = 0.0; // Cumulative raw value for stacking

        for group_df in groups.iter() {
            let group_color = self.resolve_group_color(group_df, context, &mark_config.color)?;
            let y_series = group_df.column(&y_enc.field)?.as_materialized_series();
            let y_vals_raw: Vec<f64> = y_series.f64()?.into_no_null_iter().collect();

            // Radius (X) Scale: Used for Nightingale/Rose charts
            let radius_scale = context.coord.get_y_scale();
            let radius_norms = if let Some(x_enc) = self.encoding.x.as_ref() {
                let x_series = group_df.column(&x_enc.field)?.as_materialized_series();
                Some(radius_scale.scale_type().normalize_series(radius_scale, x_series)?)
            } else {
                None
            };

            for (i, y_val_raw) in y_vals_raw.into_iter().enumerate() {
                
                // --- A. CALCULATE THETA BOUNDS (SCHEME A) ---
                let (mut theta_start_n, mut theta_end_n) = if is_stacked {
                    // Normalize raw stacked values against the total sum
                    let start_n = global_stack_pos / total_sum;
                    let end_n = (global_stack_pos + y_val_raw) / total_sum;
                    
                    global_stack_pos += y_val_raw; // Increment stack
                    (start_n, end_n)
                } else {
                    // Rose Chart: Use the scale's categorical normalization
                    let theta_scale = context.coord.get_x_scale();
                    let center_n = theta_scale.scale_type().normalize_series(theta_scale, y_series)?.get(i).unwrap_or(0.0);
                    let angular_step = 1.0 / (theta_scale.logical_max() as f64).max(1.0);
                    let half_span = (angular_step * mark_config.width) / 2.0;
                    (center_n - half_span, center_n + half_span)
                };

                // Apply Padding
                if (theta_end_n - theta_start_n) > pad_angle_norm * 2.0 {
                    theta_start_n += pad_angle_norm;
                    theta_end_n -= pad_angle_norm;
                }

                // --- B. CALCULATE RADIAL BOUNDS ---
                // IMPORTANT: r_outer_n must be a ratio [0, 1]. 
                // We multiply the data-driven radius by the mark's radius config.
                let r_inner_n = mark_config.inner_radius; 
                let r_outer_n = match &radius_norms {
                    Some(norms) => norms.get(i).unwrap_or(0.0) * mark_config.outer_radius,
                    None => mark_config.outer_radius, 
                };

                // --- C. TRANSFORM & DRAW ---
                let sector_points = vec![
                    (theta_start_n, r_inner_n),
                    (theta_start_n, r_outer_n),
                    (theta_end_n,   r_outer_n),
                    (theta_end_n,   r_inner_n),
                ];

                let pixel_points = context.transform_path(&sector_points, true);
                println!("pixel_points: {:?}", pixel_points);

                backend.draw_polygon(PolygonConfig {
                    points: pixel_points.into_iter()
                        .map(|(px, py)| (px as Precision, py as Precision))
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

impl Chart<MarkArc> {
    /// Resolves the color for a specific data group (sector).
    /// Maps the first value of the grouping column to the designated color scale.
    fn resolve_group_color(
        &self, 
        df: &DataFrame, 
        context: &PanelContext, 
        fallback: &SingleColor
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            
            // Take the first value of the series to determine the color for this slice.
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