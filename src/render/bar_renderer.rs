use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, PolygonConfig, RenderBackend, TextConfig};
use crate::encode::y::StackMode;
use crate::error::ChartonError;
use crate::mark::bar::MarkBar;
use crate::visual::color::SingleColor;
use ahash::AHashMap;

impl MarkRenderer for Chart<MarkBar> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let ds = &self.data;
        if ds.row_count == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkBar configuration is missing".into()))?;

        // --- STEP 1: Encoding & Scales ---
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

        let is_stacked = y_enc.stack != StackMode::None;
        let is_pie_mode = x_enc.field.is_empty();
        let hints = context.coord.layout_hints();
        let is_polar = hints.needs_interpolation;
        let needs_nightingale_sqrt = is_polar && !is_pie_mode;

        // --- STEP 2: Deterministic X-Mapping for Stack Accumulator ---
        // Crucial: Use unique_values() to create a stable index for each X-category.
        // This prevents stack_acc from drifting if row order changes.
        let x_uniques = ds.column(&x_enc.field)?.unique_values();
        let mut x_idx_map = AHashMap::with_capacity(x_uniques.len());
        for (i, val) in x_uniques.iter().enumerate() {
            x_idx_map.insert(val, i);
        }

        // Accumulator size corresponds to unique X locations, initialized to 0.0.
        let mut stack_acc = vec![0.0; x_uniques.len()];

        // --- STEP 3: Vectorized Data Extraction ---
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_values = ds.column(&y_enc.field)?.to_f64_vec();

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

        // Pie mode total for percentage labels
        let global_total = if is_pie_mode {
            y_values.iter().sum::<f64>().max(1.0)
        } else {
            1.0
        };

        // --- STEP 4: Grouping & Dodging Logic ---
        let color_field = context
            .spec
            .aesthetics
            .color
            .as_ref()
            .map(|c| c.field.as_str());
        let grouped_data = ds.group_by(color_field);

        let n_groups = if is_pie_mode {
            1.0
        } else {
            grouped_data.groups.len() as f64
        };

        // Layout Parameter Resolution
        let eff_width = mark_config.width.unwrap_or(hints.default_bar_width);
        let eff_spacing = mark_config.spacing.unwrap_or(hints.default_bar_spacing);
        let eff_span = mark_config.span.unwrap_or(hints.default_bar_span);
        let eff_stroke = mark_config.stroke.unwrap_or(hints.default_bar_stroke);
        let eff_stroke_width = mark_config
            .stroke_width
            .unwrap_or(hints.default_bar_stroke_width);

        let unit_step_norm = (x_scale.normalize(1.0) - x_scale.normalize(0.0)).abs();
        let bar_width_data = if is_stacked || n_groups <= 1.0 {
            eff_width.min(eff_span)
        } else {
            eff_span / (n_groups + (n_groups - 1.0) * eff_spacing)
        };

        let bar_width_norm = bar_width_data * unit_step_norm;
        let spacing_norm = bar_width_norm * eff_spacing;

        // --- STEP 5: Rendering Loop ---
        for (group_idx, (_name, row_indices)) in grouped_data.groups.iter().enumerate() {
            for &idx in row_indices {
                let y_val = y_values[idx];

                // Skip rendering empty bars in non-stacked mode for performance
                if y_val == 0.0 && !is_stacked {
                    continue;
                }

                // Resolve the specific X-category for this row to get the correct stack baseline
                let x_str = ds.get_str_or(&x_enc.field, idx, "null");
                let x_pos_idx = *x_idx_map.get(&x_str).unwrap_or(&0);
                let x_tick_n = x_norms[idx].unwrap_or(0.0);

                // A: Resolve Y-Bounds (Radius/Height) with Stack Logic
                let (y_low_n, y_high_n) = if is_stacked {
                    let start = stack_acc[x_pos_idx];
                    let end = start + y_val;
                    stack_acc[x_pos_idx] = end; // Update baseline for next color in this X-group

                    if needs_nightingale_sqrt {
                        (
                            y_scale.normalize(start).sqrt(),
                            y_scale.normalize(end).sqrt(),
                        )
                    } else {
                        (y_scale.normalize(start), y_scale.normalize(end))
                    }
                } else {
                    let n_val = y_scale.normalize(y_val);
                    let final_n = if needs_nightingale_sqrt {
                        n_val.sqrt()
                    } else {
                        n_val
                    };
                    (y_scale.normalize(0.0), final_n)
                };

                // B: Resolve X-Position (Angular/Width) with Dodging
                let offset_norm = if !is_stacked && n_groups > 1.0 {
                    (group_idx as f64 - (n_groups - 1.0) / 2.0) * (bar_width_norm + spacing_norm)
                } else {
                    0.0
                };

                let x_center_n = x_tick_n + offset_norm;
                let left_n = x_center_n - bar_width_norm / 2.0;
                let right_n = x_center_n + bar_width_norm / 2.0;

                // C: Geometric Construction
                let rect_path = if is_pie_mode {
                    // Pie mode: X maps to Radius, Y maps to Angle
                    vec![
                        (y_low_n, left_n),
                        (y_low_n, right_n),
                        (y_high_n, right_n),
                        (y_high_n, left_n),
                    ]
                } else {
                    // Standard Bar: X maps to X, Y maps to Y
                    vec![
                        (left_n, y_low_n),
                        (left_n, y_high_n),
                        (right_n, y_high_n),
                        (right_n, y_low_n),
                    ]
                };

                let pixel_points: Vec<(Precision, Precision)> = if hints.needs_interpolation {
                    context
                        .transform_path(&rect_path, true)
                        .into_iter()
                        .map(|(px, py)| (px as Precision, py as Precision))
                        .collect()
                } else {
                    rect_path
                        .iter()
                        .map(|&(nx, ny)| {
                            let (px, py) = context.coord.transform(nx, ny, &context.panel);
                            (px as Precision, py as Precision)
                        })
                        .collect()
                };

                // D: Visual Resolution
                let color_val = color_norms.as_ref().and_then(|cn| cn[idx]);
                let final_color =
                    self.resolve_color_from_value(color_val, context, &mark_config.color);

                backend.draw_polygon(PolygonConfig {
                    points: pixel_points,
                    fill: final_color,
                    stroke: eff_stroke,
                    stroke_width: eff_stroke_width as Precision,
                    fill_opacity: mark_config.opacity as Precision,
                    stroke_opacity: mark_config.opacity as Precision,
                });

                // E: Pie Mode Labels
                if is_pie_mode {
                    self.render_pie_label(
                        y_val,
                        global_total,
                        y_low_n,
                        y_high_n,
                        left_n,
                        right_n,
                        y_enc.normalize,
                        context,
                        backend,
                        mark_config.opacity as Precision,
                    );
                }
            }
        }

        Ok(())
    }
}

impl Chart<MarkBar> {
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

    fn render_pie_label(
        &self,
        y_val: f64,
        total: f64,
        y_low_n: f64,
        y_high_n: f64,
        left_n: f64,
        right_n: f64,
        is_normalized: bool,
        context: &PanelContext,
        backend: &mut dyn RenderBackend,
        opacity: f32,
    ) {
        let percentage = if is_normalized {
            y_val * 100.0
        } else {
            (y_val / total) * 100.0
        };

        if percentage > 3.0 {
            let label_text = format!("{:.1}%", percentage);
            let mid_angle_n = (y_low_n + y_high_n) / 2.0;
            let mid_radius_n = (left_n + right_n) / 2.0;
            let (lx, ly) = context
                .coord
                .transform(mid_angle_n, mid_radius_n, &context.panel);
            let theme = &context.spec.theme;

            backend.draw_text(TextConfig {
                x: lx as Precision,
                y: ly as Precision,
                text: label_text,
                font_size: (theme.tick_label_size - 1.0) as Precision,
                font_family: theme.tick_label_family.clone(),
                color: SingleColor::new("white"),
                text_anchor: "middle".into(),
                font_weight: "bold".into(),
                opacity: opacity as Precision,
            });
        }
    }
}
