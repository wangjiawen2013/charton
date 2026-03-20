use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RectConfig, RenderBackend};
use crate::error::ChartonError;
use crate::mark::tick::MarkTick;
use crate::visual::color::SingleColor;

// ============================================================================
// MARK RENDERING (The main data-to-geometry loop)
// ============================================================================

impl MarkRenderer for Chart<MarkTick> {
    /// Orchestrates the transformation of raw data rows into visual tick geometries.
    ///
    /// Ticks are rendered as thin rectangles centered on the data point position.
    /// By default, ticks are vertical (perpendicular to x-axis). When users swap
    /// x/y encodings, the ticks automatically become horizontal.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;

        // Return early if there is no data to render.
        if df_source.df.height() == 0 {
            return Ok(());
        }

        // --- STEP 1: ENCODING VALIDATION ---
        let x_enc = self.encoding.x.as_ref().ok_or_else(|| {
            ChartonError::Encoding("X-axis encoding is missing from specification".to_string())
        })?;
        let y_enc = self.encoding.y.as_ref().ok_or_else(|| {
            ChartonError::Encoding("Y-axis encoding is missing from specification".to_string())
        })?;

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkTick configuration is missing".to_string()))?;

        // --- STEP 2: POSITION NORMALIZATION (Vectorized) ---
        let x_series = df_source.column(&x_enc.field)?;
        let y_series = df_source.column(&y_enc.field)?;

        let x_scale_trait = context.coord.get_x_scale();
        let y_scale_trait = context.coord.get_y_scale();

        let x_norms = x_scale_trait
            .scale_type()
            .normalize_series(x_scale_trait, &x_series)?;
        let y_norms = y_scale_trait
            .scale_type()
            .normalize_series(y_scale_trait, &y_series)?;

        // --- STEP 3: COLOR MAPPING ---
        let color_iter: Box<dyn Iterator<Item = SingleColor>> =
            if let Some(ref mapping) = context.spec.aesthetics.color {
                let s = df_source.column(&mapping.field)?;
                let s_trait = mapping.scale_impl.as_ref();

                let norms = s_trait.scale_type().normalize_series(s_trait, &s)?;
                let l_max = s_trait.logical_max();

                let color_vec: Vec<SingleColor> = norms
                    .into_iter()
                    .map(|opt_n| {
                        s_trait
                            .mapper()
                            .map(|m| m.map_to_color(opt_n.unwrap_or(0.0), l_max))
                            .unwrap_or_else(|| SingleColor::from("#333333"))
                    })
                    .collect();
                Box::new(color_vec.into_iter())
            } else {
                Box::new(std::iter::repeat(mark_config.color))
            };

        // --- STEP 4: GEOMETRY PROJECTION & RENDERING ---
        let stroke_color = mark_config.color;
        let thickness = mark_config.thickness;
        let band_size = mark_config.band_size;
        let opacity = mark_config.opacity;
        let is_flipped = context.coord.is_flipped();

        // Zip all streams into a single loop to emit draw calls for each row.
        for (((x_n, y_n), fill_color), _) in x_norms
            .into_iter()
            .zip(y_norms.into_iter())
            .zip(color_iter)
            .zip(std::iter::repeat(()))
        {
            let x_norm = x_n.unwrap_or(0.0);
            let y_norm = y_n.unwrap_or(0.0);

            let (px, py) = context.transform(x_norm, y_norm);

            // Calculate tick rectangle geometry based on flip state
            let (rect_x, rect_y, rect_width, rect_height) = if !is_flipped {
                // Vertical ticks: thin width, extended height
                let half_thickness = thickness / 2.0;
                let half_band = band_size / 2.0;
                (px - half_thickness, py - half_band, thickness, band_size)
            } else {
                // Horizontal ticks: extended width, thin height
                let half_thickness = thickness / 2.0;
                let half_band = band_size / 2.0;
                (px - half_band, py - half_thickness, band_size, thickness)
            };

            let rect_config = RectConfig {
                x: rect_x as Precision,
                y: rect_y as Precision,
                width: rect_width as Precision,
                height: rect_height as Precision,
                fill: fill_color,
                stroke: stroke_color.clone(),
                stroke_width: 0.0,
                opacity: opacity as Precision,
            };

            backend.draw_rect(rect_config);
        }

        Ok(())
    }
}
