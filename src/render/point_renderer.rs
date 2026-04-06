use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{
    CircleConfig, MarkRenderer, PointElementConfig, PolygonConfig, RectConfig, RenderBackend,
};
use crate::core::utils::Parallelizable;
use crate::error::ChartonError;
use crate::mark::point::MarkPoint;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

// ============================================================================
// MARK RENDERING (High-Performance Parallel Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkPoint> {
    /// Orchestrates the transformation of raw data into visual geometries.
    /// Uses group-based parallel processing to ensure deterministic Z-indexing
    /// and appearance-based color mapping.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        let row_count = df_source.height();

        if row_count == 0 {
            return Ok(());
        }

        // --- STEP 1: SPECIFICATION VALIDATION ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding is missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y-axis encoding is missing".into()))?;
        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkPoint configuration is missing".into()))?;

        // --- STEP 2: POSITION & AESTHETIC NORMALIZATION ---
        // Vectorized normalization via Polars columns.
        let x_norms = context
            .coord
            .get_x_scale()
            .scale_type()
            .normalize_column(context.coord.get_x_scale(), df_source.column(&x_enc.field)?);
        let y_norms = context
            .coord
            .get_y_scale()
            .scale_type()
            .normalize_column(context.coord.get_y_scale(), df_source.column(&y_enc.field)?);

        // Pre-normalize aesthetics if mappings exist.
        let color_norms = context.spec.aesthetics.color.as_ref().map(|m| {
            let s = m.scale_impl.as_ref();
            s.scale_type()
                .normalize_column(s, &df_source.column(&m.field).unwrap())
        });

        let size_norms = context.spec.aesthetics.size.as_ref().map(|m| {
            let s = m.scale_impl.as_ref();
            s.scale_type()
                .normalize_column(s, &df_source.column(&m.field).unwrap())
        });

        let shape_norms = context.spec.aesthetics.shape.as_ref().map(|m| {
            let s = m.scale_impl.as_ref();
            s.scale_type()
                .normalize_column(s, &df_source.column(&m.field).unwrap())
        });

        // --- STEP 3: GROUPING (Determines Z-Index & Category Color) ---
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());
        let grouped_data = df_source.group_by(color_field);
        let palette = &context.spec.theme.palette;

        // --- STEP 4: MULTI-CORE PROCESSING PER GROUP ---
        // We iterate sequentially through groups to respect "Order of Appearance" for Z-index,
        // but process individual points within each group in parallel.
        for (group_idx, (_name, row_indices)) in grouped_data.groups.iter().enumerate() {
            // Resolve base color for this group (used for categorical grouping).
            let base_group_color = if color_field.is_some() {
                palette.get_color(group_idx)
            } else {
                mark_config.color
            };

            // Calculate geometries in parallel.
            let render_configs: Vec<PointElementConfig> = row_indices
                .maybe_par_iter()
                .filter_map(|&i| {
                    let x_n = x_norms[i]?;
                    let y_n = y_norms[i]?;

                    // Convert normalized [0,1] to screen pixels.
                    let (px, py) = context.coord.transform(x_n, y_n, &context.panel);

                    // Resolve aesthetics (Priority: Scale Mapping > Group Color > Mark Default).
                    let fill = if let Some(ref norms) = color_norms {
                        self.resolve_color_from_value(norms[i], context, &base_group_color)
                    } else {
                        base_group_color
                    };

                    let size = if let Some(ref norms) = size_norms {
                        self.resolve_size_from_value(norms[i], context, mark_config.size)
                    } else {
                        mark_config.size
                    };

                    let shape = if let Some(ref norms) = shape_norms {
                        self.resolve_shape_from_value(norms[i], context, mark_config.shape)
                    } else {
                        mark_config.shape
                    };

                    Some(PointElementConfig {
                        x: px,
                        y: py,
                        shape,
                        size,
                        fill,
                        stroke: mark_config.stroke,
                        stroke_width: mark_config.stroke_width,
                        opacity: mark_config.opacity,
                    })
                })
                .collect();

            // --- STEP 5: SEQUENTIAL DRAW DISPATCH ---
            // Render the points for this group onto the backend.
            for config in render_configs {
                self.emit_draw_call(backend, config);
            }
        }

        Ok(())
    }
}

// ============================================================================
// HELPER METHODS & GEOMETRY DISPATCH
// ============================================================================

impl Chart<MarkPoint> {
    /// Maps a normalized value to a color using the registered scale mapper.
    fn resolve_color_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> SingleColor {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.color) {
            let s_trait = mapping.scale_impl.as_ref();
            s_trait
                .mapper()
                .as_ref()
                .map(|m| m.map_to_color(v, s_trait.logical_max()))
                .unwrap_or(*fallback)
        } else {
            *fallback
        }
    }

    /// Maps a normalized value to a point size.
    fn resolve_size_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: f64,
    ) -> f64 {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.size) {
            mapping
                .scale_impl
                .mapper()
                .as_ref()
                .map(|m| m.map_to_size(v))
                .unwrap_or(fallback)
        } else {
            fallback
        }
    }

    /// Maps a normalized value to a specific PointShape.
    fn resolve_shape_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: PointShape,
    ) -> PointShape {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.shape) {
            let s_trait = mapping.scale_impl.as_ref();
            mapping
                .scale_impl
                .mapper()
                .as_ref()
                .map(|m| m.map_to_shape(v, s_trait.logical_max()))
                .unwrap_or(fallback)
        } else {
            fallback
        }
    }

    /// Dispatches the appropriate backend draw call for the given PointShape.
    fn emit_draw_call(&self, backend: &mut dyn RenderBackend, config: PointElementConfig) {
        let PointElementConfig {
            x,
            y,
            shape,
            size,
            fill,
            stroke,
            stroke_width,
            opacity,
        } = config;

        match shape {
            PointShape::Circle => {
                backend.draw_circle(CircleConfig {
                    x: x as Precision,
                    y: y as Precision,
                    radius: size as Precision,
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    opacity: opacity as Precision,
                });
            }
            PointShape::Square => {
                let side = size * 2.0;
                backend.draw_rect(RectConfig {
                    x: (x - size) as Precision,
                    y: (y - size) as Precision,
                    width: side as Precision,
                    height: side as Precision,
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    opacity: opacity as Precision,
                });
            }
            _ => {
                let (sides, rotation, scale_adj) = match shape {
                    PointShape::Diamond => (4, 0.0, 1.2),
                    PointShape::Triangle => (3, -std::f64::consts::FRAC_PI_2, 1.1),
                    PointShape::Pentagon => (5, -std::f64::consts::FRAC_PI_2, 1.0),
                    PointShape::Hexagon => (6, 0.0, 1.0),
                    PointShape::Octagon => (8, std::f64::consts::FRAC_PI_8, 1.0),
                    _ => (0, 0.0, 0.0),
                };

                let points = if shape == PointShape::Star {
                    self.calculate_star(x as f64, y as f64, size * 1.2, size * 0.5, 5)
                } else {
                    self.calculate_polygon(x as f64, y as f64, size * scale_adj, sides, rotation)
                };

                backend.draw_polygon(PolygonConfig {
                    points: points
                        .iter()
                        .map(|p| (p.0 as Precision, p.1 as Precision))
                        .collect(),
                    fill,
                    stroke,
                    stroke_width: stroke_width as Precision,
                    fill_opacity: opacity as Precision,
                    stroke_opacity: 1.0,
                });
            }
        }
    }

    fn calculate_polygon(
        &self,
        cx: f64,
        cy: f64,
        r: f64,
        sides: usize,
        rot: f64,
    ) -> Vec<(f64, f64)> {
        (0..sides)
            .map(|i| {
                let angle = rot + 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
                (cx + r * angle.cos(), cy + r * angle.sin())
            })
            .collect()
    }

    fn calculate_star(
        &self,
        cx: f64,
        cy: f64,
        out_r: f64,
        in_r: f64,
        pts: usize,
    ) -> Vec<(f64, f64)> {
        (0..(pts * 2))
            .map(|i| {
                let angle =
                    -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * (i as f64) / (pts as f64);
                let r = if i % 2 == 0 { out_r } else { in_r };
                (cx + r * angle.cos(), cy + r * angle.sin())
            })
            .collect()
    }
}
