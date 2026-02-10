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

        // --- STEP 2: Resolve Coordinate Layout Hints ---
        // We fetch the "Aesthetic Hints" from the coordinate system.
        // This tells us if we should default to thin bars (Cartesian) or wide sectors (Polar).
        let hints = context.coord.layout_hints();

        // --- STEP 3: Parameter Resolution ---
        // If the user hasn't provided a specific override (they are None), 
        // we fall back to the coordinate system's smart defaults.
        let eff_width   = mark_config.width.unwrap_or(hints.default_bar_width);
        let eff_spacing = mark_config.spacing.unwrap_or(hints.default_bar_spacing);
        let eff_span    = mark_config.span.unwrap_or(hints.default_bar_span);

        // Resolve the stroke color: User override -> Coord hint -> Fallback (black)
        let eff_stroke = mark_config.stroke.clone().unwrap_or(hints.default_bar_stroke.clone());

        // --- STEP 4: Calculate Unit Step in Normalized Space ---
        let n0 = x_scale.normalize(0.0);
        let n1 = x_scale.normalize(1.0);
        let unit_step_norm = (n1 - n0).abs();

        // --- STEP 5: Grouping & Dodge Meta ---
        let groups = match color_field {
            Some(col) => df.partition_by_stable([col], true)?,
            None => vec![df.clone()],
        };
        let n_groups = groups.len() as f64;

        // Calculate logical bar width considering grouping/dodging.
        let bar_width_data = if is_stacked || n_groups <= 1.0 {
            eff_width.min(eff_span)
        } else {
            // Formula accounts for group span, number of groups, and intra-group spacing.
            eff_width.min(
                eff_span / (n_groups + (n_groups - 1.0) * eff_spacing)
            )
        };

        let bar_width_norm = bar_width_data * unit_step_norm;
        let spacing_norm = bar_width_norm * eff_spacing;

        let mut stack_acc = Vec::new();

        // --- STEP 6: Render Loop ---
        for (group_idx, group_df) in groups.iter().enumerate() {
            let group_color = self.resolve_group_color(group_df, context, &mark_config.color)?;
            
            let x_series = group_df.column(&x_enc.field)?.as_materialized_series();
            let y_series = group_df.column(&y_enc.field)?.as_materialized_series();

            let x_norms = x_scale.scale_type().normalize_series(x_scale, x_series)?;
            let y_vals: Vec<f64> = y_series.f64()?.into_no_null_iter().collect();

            for (i, (opt_x_n, y_val)) in x_norms.into_iter().zip(y_vals).enumerate() {
                let x_tick_n = opt_x_n.unwrap_or(0.0);

                let (y_low_n, y_high_n) = if is_stacked {
                    if stack_acc.len() <= i { stack_acc.push(0.0); }
                    let start = stack_acc[i];
                    let end = start + y_val;
                    stack_acc[i] = end;
                    (y_scale.normalize(start), y_scale.normalize(end))
                } else {
                    (y_scale.normalize(0.0), y_scale.normalize(y_val))
                };

                let offset_norm = if !is_stacked && n_groups > 1.0 {
                    (group_idx as f64 - (n_groups - 1.0) / 2.0) * (bar_width_norm + spacing_norm)
                } else {
                    0.0
                };

                let x_center_n = x_tick_n + offset_norm;
                let left_n = x_center_n - bar_width_norm / 2.0;
                let right_n = x_center_n + bar_width_norm / 2.0;

                // --- GEOMETRIC TRANSFORMATION ---
                // We define the bar as a 4-point rectangle in Normalized Space.
                let rect_path = vec![
                    (left_n, y_low_n),   // Bottom-Left
                    (left_n, y_high_n),  // Top-Left
                    (right_n, y_high_n), // Top-Right
                    (right_n, y_low_n),  // Bottom-Right
                ];

                // Performance Optimization: The Fast-Path vs High-Accuracy-Path.
                let pixel_points = if hints.needs_interpolation {
                    // POLAR/GEO PATH: Distorts straight lines into curves.
                    // We call transform_path to perform adaptive point insertion.
                    context.transform_path(&rect_path, true)
                } else {
                    // CARTESIAN PATH: Fast linear mapping.
                    // Straight lines in normalized space remain straight in pixel space.
                    // We only need to transform the 4 vertices.
                    rect_path.iter()
                        .map(|(nx, ny)| context.coord.transform(*nx, *ny, &context.panel))
                        .collect::<Vec<_>>()
                };

                backend.draw_polygon(PolygonConfig {
                    points: pixel_points.into_iter()
                        .map(|(x, y)| (x as Precision, y as Precision))
                        .collect(),
                    fill: group_color,
                    stroke: eff_stroke,
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