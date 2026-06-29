use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, PathConfig, PathTopology, RenderBackend};
use crate::core::utils::Parallelizable;
use crate::error::ChartonError;
use crate::mark::geo_path::MarkGeoPath;
use crate::visual::color::SingleColor;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl MarkRenderer for Chart<MarkGeoPath> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let ds = &self.data;
        if ds.row_count == 0 {
            return Ok(());
        }

        let mark_config = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Mark("MarkGeoPath configuration is missing".to_string())
        })?;

        // --- STEP 1: Extract Required Encodings ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or(ChartonError::Encoding("X (longitude) missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or(ChartonError::Encoding("Y (latitude) missing".into()))?;

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // --- STEP 2: Extract PathGroup encoding ---
        let group_field = self
            .encoding
            .path_group
            .as_ref()
            .map(|pg| pg.field.as_str());

        // --- STEP 3: Normalize coordinate columns ---
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_enc.field)?);

        // --- STEP 4: Resolve color mapping ---
        let color_norms = if let Some(ref color_map) = context.spec.aesthetics.color {
            Some(
                color_map
                    .scale_impl
                    .scale_type()
                    .normalize_column(color_map.scale_impl.as_ref(), ds.column(&color_map.field)?),
            )
        } else {
            None
        };

        // --- STEP 5: Group rows by PathGroup ---
        let grouped_data = ds.group_by(group_field);

        // --- STEP 6: Build and render each polygon ---
        let geo_render_data: Vec<_> = grouped_data
            .groups
            .maybe_par_iter()
            .filter_map(|(_group_name, row_indices)| {
                if row_indices.len() < 2 {
                    return None;
                }

                // 6.1 Build normalized point sequence in row order
                let norm_points: Vec<(f64, f64)> = row_indices
                    .iter()
                    .filter_map(|&idx| {
                        let xn = x_norms[idx]?;
                        let yn = y_norms[idx]?;
                        Some((xn, yn))
                    })
                    .collect();

                if norm_points.len() < 2 {
                    return None;
                }

                // 6.2 Transform through coordinate system
                let pixel_points = context.transform_path(&norm_points, true);

                let render_points: Vec<(Precision, Precision)> = pixel_points
                    .into_iter()
                    .map(|(px, py)| (px as Precision, py as Precision))
                    .collect();

                // 6.3 Resolve fill color
                let fill_color = if let (Some(cn), Some(first_idx)) =
                    (color_norms.as_ref(), row_indices.first())
                {
                    resolve_fill_color(*first_idx, cn, context, &mark_config.fill)
                } else {
                    mark_config.fill
                };

                Some((render_points, fill_color))
            })
            .collect();

        // --- STEP 7: Dispatch to render backend ---
        for (points, fill_color) in geo_render_data {
            backend.draw_path(PathConfig {
                points: points.clone(),
                fill: fill_color,
                stroke: mark_config.stroke,
                stroke_width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
                dash: vec![],
                topology: PathTopology::Complex,
            });

            if mark_config.stroke_width > 0.0 && mark_config.stroke != SingleColor::none() {
                backend.draw_path(PathConfig {
                    points,
                    fill: SingleColor::none(),
                    stroke: mark_config.stroke,
                    stroke_width: mark_config.stroke_width as Precision,
                    opacity: 1.0,
                    dash: vec![],
                    topology: PathTopology::Simple,
                });
            }
        }

        Ok(())
    }
}

/// Resolves the fill color for a polygon based on its associated data value.
fn resolve_fill_color(
    first_idx: usize,
    color_norms: &[Option<f64>],
    context: &PanelContext,
    fallback: &SingleColor,
) -> SingleColor {
    if let Some(val) = color_norms[first_idx] {
        if let Some(mapping) = &context.spec.aesthetics.color {
            let s_trait = mapping.scale_impl.as_ref();
            s_trait
                .mapper()
                .as_ref()
                .map(|m| m.map_to_color(val, s_trait.logical_max()))
                .unwrap_or(*fallback)
        } else {
            *fallback
        }
    } else {
        *fallback
    }
}

impl Chart<MarkGeoPath> {
    /// Provides access to the path_group encoding field if set.
    pub fn get_path_group_field(&self) -> Option<&str> {
        self.encoding
            .path_group
            .as_ref()
            .map(|pg| pg.field.as_str())
    }
}
