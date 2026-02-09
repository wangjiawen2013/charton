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

        // --- STEP 1: Scale & Encoding Setup ---
        // Theta (Y) is mandatory as it defines the angular distribution.
        // Radius (X) is now truly optional: 
        // - None: Standard Pie Chart (constant radius).
        // - Some: Rose/Nightingale Chart (variable radius).
        let y_enc = self.encoding.y.as_ref().ok_or(ChartonError::Encoding("Theta (Y) missing".into()))?;
        let x_enc_opt = self.encoding.x.as_ref(); 
        
        let y_scale = context.coord.get_y_scale();
        let is_stacked = y_enc.stack;

        // --- STEP 2: Partitioning ---
        // Group data by the color field to render distinct sectors (slices).
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());
        let groups = match color_field {
            Some(col) => df.partition_by_stable([col], true)?,
            None => vec![df.clone()],
        };

        // --- STEP 3: Geometry Pre-calculations ---
        // Normalize the padding angle from radians to a [0, 1] relative unit 
        // representing a fraction of the full 2π circle.
        let pad_angle_norm = mark_config.pad_angle / (2.0 * std::f64::consts::PI);

        // For Rose charts (Discrete Theta), calculate the normalized width of one 
        // categorical slot (e.g., if there are 4 directions, each gets 0.25).
        let angular_step_norm = if !is_stacked {
            1.0 / (y_scale.logical_max() as f64).max(1.0)
        } else {
            0.0
        };

        // Stack accumulator: used in Pie/Nightingale modes to keep track of 
        // the starting angle of the next sector.
        let mut stack_acc = Vec::new();

        // --- STEP 4: Render Loop ---
        for group_df in groups.iter() {
            // Resolve the aesthetic color for this specific group/sector.
            let group_color = self.resolve_group_color(group_df, context, &mark_config.color)?;
            
            let y_series = group_df.column(&y_enc.field)?.as_materialized_series();

            // Radius (X) Normalization:
            // Only perform data lookup if the Radius encoding exists.
            let x_norms = if let Some(x_enc) = x_enc_opt {
                let x_series = group_df.column(&x_enc.field)?.as_materialized_series();
                let x_scale = context.coord.get_x_scale();
                Some(x_scale.scale_type().normalize_series(x_scale, x_series)?)
            } else {
                None
            };
            
            // Theta (Y) Normalization:
            // Rose charts use the normalized index; Pie charts use raw values for stacking.
            let y_norms = if !is_stacked {
                Some(y_scale.scale_type().normalize_series(y_scale, y_series)?)
            } else {
                None
            };
            let y_vals_raw: Vec<f64> = y_series.f64()?.into_no_null_iter().collect();

            for (i, y_val_raw) in y_vals_raw.into_iter().enumerate() {
                
                // --- Determine Angular Bounds (Theta) ---
                let (mut theta_start_n, mut theta_end_n) = if is_stacked {
                    // PIE CHART MODE: Stack raw values to find cumulative bounds, then normalize.
                    if stack_acc.len() <= i { stack_acc.push(0.0); }
                    let start = stack_acc[i];
                    let end = start + y_val_raw;
                    stack_acc[i] = end;
                    (y_scale.normalize(start), y_scale.normalize(end))
                } else {
                    // ROSE CHART MODE: Use the centered normalized position for the discrete category.
                    let center_n = y_norms.as_ref().and_then(|ca| ca.get(i)).unwrap_or(0.0);
                    let half_span = (angular_step_norm * mark_config.width) / 2.0;
                    (center_n - half_span, center_n + half_span)
                };

                // Apply Padding: Shave a small angular gap from the edges of the sector.
                if (theta_end_n - theta_start_n) > pad_angle_norm * 2.0 {
                    theta_start_n += pad_angle_norm;
                    theta_end_n -= pad_angle_norm;
                }

                // --- Determine Radial Bounds ---
                let r_inner_n = mark_config.inner_radius;
                let r_outer_n = match &x_norms {
                    Some(norms) => {
                        // VARIABLE RADIUS: Map the data value to radial length.
                        norms.get(i).unwrap_or(0.0) * mark_config.outer_radius
                    },
                    None => {
                        // CONSTANT RADIUS: Use the maximum configured outer radius.
                        mark_config.outer_radius
                    }
                };

                // --- STEP 5: Coordinate Transformation ---
                // Define the sector as a "rectangle" in Polar Space: (Radius, Theta).
                // transform_path will convert this into a curved polygon in Pixel Space.
                let sector_rect = vec![
                    (r_inner_n, theta_start_n), // Inner start
                    (r_outer_n, theta_start_n), // Outer start
                    (r_outer_n, theta_end_n),   // Outer end
                    (r_inner_n, theta_end_n),   // Inner end
                ];

                // Setting 'closed=true' ensures the path returns to the first point.
                let pixel_points = context.transform_path(&sector_rect, true);

                // --- STEP 6: Drawing ---
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