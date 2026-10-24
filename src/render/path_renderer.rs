//! Renderer for open paths and closed polygons.
//!
//! Both marks connect rows that share a `path_group` in row order. A
//! `path_group` may contain several subpaths: `path_group(...).with_subpath(...)`
//! separates an outer ring from its holes (or the parts of a multi-line). The
//! only geometric difference between the two marks is closure: an open path
//! stops at its last vertex and is stroked, while a polygon joins each ring
//! back to its first vertex and fills the even-odd interior.
//!
//! The shared plumbing lives here so the two marks cannot drift: grouping,
//! colour resolution, subpath splitting and the coordinate transform are
//! identical.

use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::data::Dataset;
use crate::core::layer::{MarkRenderer, PathConfig, PathTopology, RenderBackend};
use crate::core::semantic::{Group, Role};
use crate::core::utils::Parallelizable;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::mark::path::MarkPath;
use crate::mark::polygon::MarkPolygon;
use crate::visual::color::SingleColor;
use ahash::AHashMap;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// The style surface shared by the two path-family marks.
trait PathStyle: Mark {
    /// Whether the vertex loop is closed and filled.
    const CLOSED: bool;

    /// The fill colour, for closed marks. Open paths have no fill.
    fn fill(&self) -> Option<SingleColor>;

    fn stroke_width(&self) -> f64;

    fn dash(&self) -> &[f64];
}

impl PathStyle for MarkPath {
    const CLOSED: bool = false;

    fn fill(&self) -> Option<SingleColor> {
        None
    }

    fn stroke_width(&self) -> f64 {
        self.stroke_width
    }

    fn dash(&self) -> &[f64] {
        &self.dash
    }
}

impl PathStyle for MarkPolygon {
    const CLOSED: bool = true;

    fn fill(&self) -> Option<SingleColor> {
        Some(self.fill)
    }

    fn stroke_width(&self) -> f64 {
        self.stroke_width
    }

    fn dash(&self) -> &[f64] {
        &self.dash
    }
}

impl MarkRenderer for Chart<MarkPath> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        render_path_marks(self, backend, context)
    }
}

impl MarkRenderer for Chart<MarkPolygon> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        render_path_marks(self, backend, context)
    }
}

fn render_path_marks<T: PathStyle>(
    chart: &Chart<T>,
    backend: &mut dyn RenderBackend,
    context: &PanelContext,
) -> Result<(), ChartonError> {
    let ds = &chart.data;
    if ds.row_count == 0 {
        return Ok(());
    }

    let mark = chart.mark.as_ref().ok_or_else(|| {
        ChartonError::Mark(
            if T::CLOSED {
                "polygon mark configuration is missing"
            } else {
                "path mark configuration is missing"
            }
            .to_string(),
        )
    })?;

    // --- STEP 1: Required encodings ---
    let x_enc = chart
        .encoding
        .x
        .as_ref()
        .ok_or(ChartonError::Encoding("X missing".into()))?;
    let y_enc = chart
        .encoding
        .y
        .as_ref()
        .ok_or(ChartonError::Encoding("Y missing".into()))?;

    let x_scale = context.coord.get_x_scale();
    let y_scale = context.coord.get_y_scale();

    // --- STEP 2: Grouping ---
    let group_field = chart
        .encoding
        .path_group
        .as_ref()
        .map(|pg| pg.field.as_str());
    let subpath_field = chart
        .encoding
        .path_group
        .as_ref()
        .and_then(|pg| pg.subpath_field());

    // --- STEP 3: Normalize coordinates ---
    let x_norms = x_scale
        .scale_type()
        .normalize_column(x_scale, ds.column(&x_enc.field)?);
    let y_norms = y_scale
        .scale_type()
        .normalize_column(y_scale, ds.column(&y_enc.field)?);

    // --- STEP 4: Resolve the colour channel ---
    // Only follow the shared colour scale when *this* layer actually mapped a
    // colour. Without this check a layer that did not ask for colour (for
    // example the inner box of a violin) would be painted with whatever field
    // another layer happened to use for colour.
    let color_norms = if chart.encoding.color.is_some() {
        if let Some(ref color_map) = context.spec.aesthetics.color {
            Some(
                color_map
                    .scale_impl
                    .scale_type()
                    .normalize_column(color_map.scale_impl.as_ref(), ds.column(&color_map.field)?),
            )
        } else {
            None
        }
    } else {
        None
    };

    let has_color = chart.encoding.color.is_some();
    let stroke = mark.stroke();
    let stroke_width = mark.stroke_width() as Precision;
    let opacity = mark.opacity() as Precision;
    let dash: Vec<Precision> = mark.dash().iter().map(|&d| d as Precision).collect();
    let fallback = if T::CLOSED {
        mark.fill().unwrap_or_else(SingleColor::none)
    } else {
        stroke
    };

    // --- STEP 5: One shape per path group, one ring per subpath ---
    let grouped_data = ds.group_by(group_field);
    let render_data: Vec<_> = grouped_data
        .groups
        .maybe_par_iter()
        .filter_map(|(group_key, row_indices)| {
            if row_indices.len() < 2 {
                return None;
            }

            // 5.1 Split the group into subpaths. Without a subpath field the
            //     whole group is one ring; with it, each distinct value inside
            //     the group is an outer/inner ring (or a separate polyline).
            let subpaths: Vec<Vec<(Precision, Precision)>> =
                split_subpaths(ds, row_indices, subpath_field)
                    .into_iter()
                    .filter_map(|rows| {
                        let norm_points: Vec<(f64, f64)> = rows
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

                        // 5.2 Transform through the coordinate system. Each
                        //     subpath is closed independently when the mark is
                        //     a polygon.
                        let pixel_points = context.transform_path(&norm_points, T::CLOSED);
                        Some(
                            pixel_points
                                .into_iter()
                                .map(|(px, py)| (px as Precision, py as Precision))
                                .collect(),
                        )
                    })
                    .collect();

            if subpaths.is_empty() {
                return None;
            }

            // 5.3 Resolve the group colour.
            let group_color =
                if let (Some(cn), Some(first_idx)) = (color_norms.as_ref(), row_indices.first()) {
                    resolve_group_color(*first_idx, cn, context, &fallback)
                } else {
                    fallback
                };

            Some((group_key.clone(), subpaths, group_color))
        })
        .collect();

    // --- STEP 6: Dispatch to the backend ---
    for (group_key, subpaths, group_color) in render_data {
        // One scope per path group, so a shape (an island, a region, a
        // trajectory) can be addressed by name.
        backend.begin_group(&Group::new(Role::Series).series_opt(group_key.as_deref()));

        if T::CLOSED {
            // Fill and outline are two independent passes. The fill carries no
            // stroke, so the anti-aliased fill stays clean, and the outline
            // pass strokes every ring exactly once — closing edges included.
            backend.draw_path(PathConfig {
                fill: group_color,
                stroke: SingleColor::none(),
                stroke_width: 0.0,
                opacity,
                dash: vec![],
                topology: PathTopology::Complex,
                ..PathConfig::from_subpaths(subpaths.clone())
            });

            if stroke_width > 0.0 && stroke != SingleColor::none() {
                backend.draw_path(PathConfig {
                    fill: SingleColor::none(),
                    stroke,
                    stroke_width,
                    opacity: 1.0,
                    dash: dash.clone(),
                    topology: PathTopology::Complex,
                    ..PathConfig::from_subpaths(subpaths)
                });
            }
        } else {
            // An open path has no fill, so its colour channel drives the
            // stroke. This is what colours contour lines by their level.
            let path_stroke = if has_color { group_color } else { stroke };

            if stroke_width > 0.0 && path_stroke != SingleColor::none() {
                backend.draw_path(PathConfig {
                    fill: SingleColor::none(),
                    stroke: path_stroke,
                    stroke_width,
                    opacity,
                    dash: dash.clone(),
                    topology: PathTopology::Simple,
                    ..PathConfig::from_subpaths(subpaths)
                });
            }
        }

        backend.end_group();
    }

    Ok(())
}

/// Splits one path group's rows into subpaths.
///
/// Rows keep their original order. Subpaths are ordered by first appearance,
/// matching the grouping order used elsewhere in the pipeline. Without a
/// subpath field the entire group is one subpath.
fn split_subpaths(ds: &Dataset, rows: &[usize], field: Option<&str>) -> Vec<Vec<usize>> {
    let Some(field) = field else {
        return vec![rows.to_vec()];
    };

    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut slot_by_key: AHashMap<String, usize> = AHashMap::new();

    for &idx in rows {
        let key = ds.get(field, idx).to_string().unwrap_or_default();
        if let Some(&slot) = slot_by_key.get(&key) {
            groups[slot].push(idx);
        } else {
            slot_by_key.insert(key, groups.len());
            groups.push(vec![idx]);
        }
    }

    groups
}

/// Resolves a group's colour from the shared colour scale.
fn resolve_group_color(
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
