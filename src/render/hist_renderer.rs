use crate::core::layer::{MarkRenderer, RenderBackend, RectConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::mark::histogram::MarkHist;
use crate::visual::color::SingleColor;
use crate::error::ChartonError;
use crate::Precision;
use polars::prelude::*;

/// Implementation of `MarkRenderer` for Histogram charts.
/// 
/// This renderer handles both standard vertical histograms and flipped horizontal 
/// histograms by checking the coordinate system state.
impl MarkRenderer for Chart<MarkHist> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data.df;
        if df_source.height() == 0 {
            return Ok(());
        }

        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkHist configuration is missing".into()))?;

        // --- STEP 1: RESOLVE ENCODINGS & SCALES ---
        let x_enc = self.encoding.x.as_ref().ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self.encoding.y.as_ref().ok_or(ChartonError::Encoding("Y missing".into()))?;
        
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // --- STEP 2: GROUPING ---
        // Partition the data if a color aesthetic is present to support grouped histograms.
        let group_column = context.spec.aesthetics.color.as_ref().map(|c| c.field.as_str());
        let groups = match group_column {
            Some(col_name) => df_source.partition_by([col_name], true)?,
            None => vec![df_source.clone()],
        };

        // Calculate the physical "thickness" of the bars based on the X-axis bins.
        let bar_thickness = self.calculate_hist_bar_size(context)?;
        
        // Detect if the coordinate system is flipped (e.g., for horizontal histograms).
        let is_flipped = context.coord.is_flipped();

        // --- STEP 3: RENDER GROUPS ---
        for group_df in groups {
            let group_color = self.resolve_group_color(&group_df, context, &mark_config.color)?;

            let x_series = group_df.column(&x_enc.field)?.as_materialized_series();
            let y_series = group_df.column(&y_enc.field)?.as_materialized_series();

            // Normalize data to [0, 1] range.
            let x_norms = x_scale.scale_type().normalize_series(x_scale, x_series)?;
            let y_norms = y_scale.scale_type().normalize_series(y_scale, y_series)?;
            
            // Baseline for frequency is 0.0 in normalized space.
            let y_baseline_norm = 0.0;

            for (opt_x, opt_y) in x_norms.into_iter().zip(y_norms.into_iter()) {
                let x_n = opt_x.unwrap_or(0.0);
                let y_n = opt_y.unwrap_or(0.0);

                // Transform normalized coordinates to screen pixels.
                // In flipped mode: px corresponds to logic Y (frequency), py to logic X (bins).
                let (px, py) = context.transform(x_n, y_n);
                let (px_base, py_base) = context.transform(x_n, y_baseline_norm);

                let rect_config = if !is_flipped {
                    // --- STANDARD VERTICAL BARS ---
                    // x-axis is horizontal, bars grow upwards (or downwards).
                    let h = (py_base - py).abs();
                    RectConfig {
                        x: (px - bar_thickness / 2.0) as Precision,
                        y: py.min(py_base) as Precision,
                        width: bar_thickness as Precision,
                        height: h as Precision,
                        fill: group_color.clone(),
                        stroke: mark_config.stroke.clone(),
                        stroke_width: mark_config.stroke_width as Precision,
                        opacity: mark_config.opacity as Precision,
                    }
                } else {
                    // --- FLIPPED HORIZONTAL BARS ---
                    // x-axis is vertical (bin locations), y-axis is horizontal (frequency).
                    let w = (px - px_base).abs();
                    RectConfig {
                        x: px.min(px_base) as Precision,
                        y: (py - bar_thickness / 2.0) as Precision,
                        width: w as Precision,
                        height: bar_thickness as Precision,
                        fill: group_color.clone(),
                        stroke: mark_config.stroke.clone(),
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
    /// Calculates the consistent pixel size (thickness) for bars.
    /// 
    /// This method maps the logical width of one bin into physical pixels. 
    /// It is coordinate-aware: it returns width for vertical charts and height for horizontal charts.
    fn calculate_hist_bar_size(&self, context: &PanelContext) -> Result<f64, ChartonError> {
        let n_bins = self.encoding.x.as_ref()
            .and_then(|x| x.bins)
            .ok_or_else(|| ChartonError::Encoding("Bin count not resolved".into()))? as f64;

        let x_field = &self.encoding.x.as_ref().unwrap().field;
        let x_scale = context.coord.get_x_scale();

        let s = self.data.df.column(x_field)?.as_materialized_series();
        let v_min = s.min::<f64>()?.ok_or(ChartonError::Data("X column is empty".into()))?;
        let v_max = s.max::<f64>()?.ok_or(ChartonError::Data("X column is empty".into()))?;

        // 4. Calculate the true data-space step between bins.
        // If there are N unique bins, the distance from the first center to the last 
        // center represents (N - 1) full bin widths.
        let data_step = if n_bins > 1.0 {
            (v_max - v_min) / (n_bins - 1.0)
        } else {
            // Fallback: if only one bin, we can't measure a step. 
            // We use a default fraction of the scale's domain.
            let (d0, d1) = x_scale.domain();
            (d1 - d0) * 0.5
        };

        // Map two logical points to normalized space to measure the relative span.
        let norm0 = x_scale.normalize(v_min);
        let norm1 = x_scale.normalize(v_min + data_step);

        // Convert normalized positions to pixels.
        let (p0_x, p0_y) = context.transform(norm0, 0.0);
        let (p1_x, p1_y) = context.transform(norm1, 0.0);

        // Determine the pixel distance. If flipped, the bin direction is vertical (Y).
        let theoretical_thickness = if context.coord.is_flipped() {
            (p1_y - p0_y).abs()
        } else {
            (p1_x - p0_x).abs()
        };
        
        // Return with a visual gap factor (0.95) to prevent bar overlap.
        Ok(theoretical_thickness * 0.95)
    }

    /// Resolves the fill color for a group based on the color encoding or fallback.
    fn resolve_group_color(
        &self, 
        df: &DataFrame, 
        context: &PanelContext, 
        fallback: &SingleColor
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            
            // Representative color for the group based on the first occurrence.
            let first_val_norm = s_trait.scale_type().normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = first_val_norm.get(0).unwrap_or(0.0);
            
            Ok(s_trait.mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| fallback.clone()))
        } else {
            Ok(fallback.clone())
        }
    }
}