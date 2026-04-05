use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RectConfig, RenderBackend};
use crate::error::ChartonError;
use crate::mark::histogram::MarkHist;
use crate::visual::color::SingleColor;

// ============================================================================
// MARK RENDERING (Histogram Implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkHist> {
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
            .ok_or_else(|| ChartonError::Mark("MarkHist configuration is missing".into()))?;

        // --- STEP 1: RESOLVE ENCODINGS & SCALES ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y-axis missing".into()))?;

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();
        let is_flipped = context.coord.is_flipped();

        // --- STEP 2: PRE-COMPUTE NORMALIZED COLUMNS ---
        // We use normalize_column to process all data points upfront.
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_enc.field)?);

        // Optional Color normalization
        let color_norms = if let Some(ref mapping) = context.spec.aesthetics.color {
            let s_trait = mapping.scale_impl.as_ref();
            Some(
                s_trait
                    .scale_type()
                    .normalize_column(s_trait, ds.column(&mapping.field)?),
            )
        } else {
            None
        };

        // --- STEP 3: CALCULATE BAR GEOMETRY ---
        // Bar thickness is calculated based on the bin count resolved in the encoding phase.
        let bar_thickness = self.calculate_hist_bar_size(context)?;
        let y_baseline_norm = 0.0; // Frequency baseline is always 0 in normalized space.

        // --- STEP 4: GROUPING & RENDERING ---
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());
        let grouped_data = ds.group_by(color_field);

        for (_key, row_indices) in &grouped_data.groups {
            for &idx in row_indices {
                // Access pre-computed normalized values by index
                let (Some(xn), Some(yn)) = (x_norms[idx], y_norms[idx]) else {
                    continue;
                };

                // Transform normalized [0,1] to screen pixels
                let (px, py) = context.coord.transform(xn, yn, &context.panel);
                let (px_base, py_base) =
                    context.coord.transform(xn, y_baseline_norm, &context.panel);

                // Resolve color for the current bar
                let fill_color = if let Some(ref norms) = color_norms {
                    self.resolve_color_from_value(norms[idx], context, &mark_config.color)
                } else {
                    mark_config.color
                };

                let rect_config = if !is_flipped {
                    // --- VERTICAL BARS ---
                    let h = (py_base - py).abs();
                    RectConfig {
                        x: (px - bar_thickness / 2.0) as Precision,
                        y: py.min(py_base) as Precision,
                        width: bar_thickness as Precision,
                        height: h as Precision,
                        fill: fill_color,
                        stroke: mark_config.stroke,
                        stroke_width: mark_config.stroke_width as Precision,
                        opacity: mark_config.opacity as Precision,
                    }
                } else {
                    // --- HORIZONTAL BARS ---
                    let w = (px - px_base).abs();
                    RectConfig {
                        x: px.min(px_base) as Precision,
                        y: (py - bar_thickness / 2.0) as Precision,
                        width: w as Precision,
                        height: bar_thickness as Precision,
                        fill: fill_color,
                        stroke: mark_config.stroke,
                        stroke_width: mark_config.stroke_width as Precision,
                        opacity: mark_config.opacity as Precision,
                    }
                };

                backend.draw_rect(rect_config);
            }
        }

        Ok(())
    }
}

// --- HELPER METHODS ---

impl Chart<MarkHist> {
    /// Determines the pixel thickness of bars by measuring the distance
    /// between adjacent bin centers in the current coordinate system.
    fn calculate_hist_bar_size(&self, context: &PanelContext) -> Result<f64, ChartonError> {
        let x_enc = self.encoding.x.as_ref().unwrap();
        let n_bins = x_enc
            .bins
            .ok_or_else(|| ChartonError::Encoding("Bin count not resolved".into()))?
            as f64;

        let x_scale = context.coord.get_x_scale();
        let col = self.data.column(&x_enc.field)?;

        // --- OPTIMIZED: Use your parallel min_max method ---
        // This replaces two separate .min() and .max() calls with one parallel scan.
        let (v_min, v_max) = col.min_max();

        // Handle the case where the column might be effectively empty or invalid
        if v_min == f64::INFINITY || v_max == f64::NEG_INFINITY {
            return Err(ChartonError::Data(
                "X column is empty or contains only nulls".into(),
            ));
        }

        // Calculate logical data-space step between bins
        let data_step = if n_bins > 1.0 {
            (v_max - v_min) / (n_bins - 1.0)
        } else {
            // Fallback for single bin
            let (d0, d1) = x_scale.domain();
            (d1 - d0) * 0.5
        };

        // Map logical step to normalized space
        let norm0 = x_scale.normalize(v_min);
        let norm1 = x_scale.normalize(v_min + data_step);

        // Convert normalized span to pixels
        let (p0_x, p0_y) = context.coord.transform(norm0, 0.0, &context.panel);
        let (p1_x, p1_y) = context.coord.transform(norm1, 0.0, &context.panel);

        let theoretical_thickness = if context.coord.is_flipped() {
            (p1_y - p0_y).abs()
        } else {
            (p1_x - p0_x).abs()
        };

        // Apply a visual gap factor (0.95) to separate bars slightly
        Ok(theoretical_thickness * 0.95)
    }

    /// Resolves a SingleColor from a normalized aesthetic value.
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
}
