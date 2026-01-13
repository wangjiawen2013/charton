use crate::core::layer::{MarkRenderer, RenderBackend};
use crate::core::context::SharedRenderingContext;
use crate::chart::Chart;
use crate::mark::point::MarkPoint;
use crate::scale::{get_normalized_value, Scale};
use crate::error::ChartonError;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkPoint> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        let data = &self.data.df;
        let row_count = data.height();
        if row_count == 0 { return Ok(()); }

        // --- PRE-FETCH PHASE ---
        // 1. Coordinates: Extract Series and Scale Types
        // We assume X and Y encodings exist for a point chart.
        let x_enc = self.encoding.x.as_ref().unwrap();
        let y_enc = self.encoding.y.as_ref().unwrap();
        
        let x_series = data.column(&x_enc.field).map_err(|e| ChartonError::Data(e.to_string()))?;
        let y_series = data.column(&y_enc.field).map_err(|e| ChartonError::Data(e.to_string()))?;
        
        // Default to Linear if no scale is specified
        let x_scale_type = x_enc.scale.as_ref().unwrap_or(&Scale::Linear);
        let y_scale_type = y_enc.scale.as_ref().unwrap_or(&Scale::Linear);

        // 2. Aesthetics: Pre-fetch optional channels (Color, Size)
        // We store the tuple (Series, (ScaleTrait, Mapper), ScaleType) for fast access.
        let color_ctx = if let (Some(enc), Some(bundle)) = (&self.encoding.color, &context.aesthetics.color) {
            let series = data.column(&enc.field).ok();
            let s_type = enc.scale.as_ref().unwrap_or(&Scale::Discrete); // Colors are often Discrete by default
            Some((series, bundle, s_type))
        } else { None };

        let size_ctx = if let (Some(enc), Some(bundle)) = (&self.encoding.size, &context.aesthetics.size) {
            let series = data.column(&enc.field).ok();
            let s_type = enc.scale.as_ref().unwrap_or(&Scale::Linear); // Size is usually Linear
            Some((series, bundle, s_type))
        } else { None };

        let mark_config = self.mark.as_ref().unwrap();
        let default_fill = mark_config.color.as_ref().map(|c| c.get_color()).unwrap_or_else(|| "black".into());

        // --- RENDER LOOP ---
        for i in 0..row_count {
            // 1. Process X and Y Coordinates
            // Normalized values are obtained using the helper, then transformed to screen pixels.
            let x_norm = get_normalized_value(context.coord.get_x_scale(), x_scale_type, &x_series.get(i).unwrap());
            let y_norm = get_normalized_value(context.coord.get_y_scale(), y_scale_type, &y_series.get(i).unwrap());
            let (px, py) = context.coord.transform(x_norm, y_norm, &context.panel);

            // 2. Resolve Color
            let fill_color = if let Some((Some(series), (scale_trait, mapper), s_type)) = &color_ctx {
                let norm = get_normalized_value(scale_trait.as_ref(), s_type, &series.get(i).unwrap());
                mapper.map_color(norm)
            } else {
                default_fill.clone()
            };

            // 3. Resolve Size
            let size = if let Some((Some(series), (scale_trait, mapper), s_type)) = &size_ctx {
                let norm = get_normalized_value(scale_trait.as_ref(), s_type, &series.get(i).unwrap());
                mapper.map_size(norm)
            } else {
                mark_config.size
            };

            // 4. Emit Draw Call
            // Note: We use the coordinate transformation result (px, py) directly.
            self.emit_draw_call(
                backend,
                &mark_config.shape,
                px, py,
                size,
                &fill_color,
                &mark_config.stroke.as_ref().map(|c| c.get_color()).unwrap_or_else(|| "none".into()),
                mark_config.stroke_width,
                mark_config.opacity
            );
        }

        Ok(())
    }
}