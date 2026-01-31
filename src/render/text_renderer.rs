use crate::core::layer::{MarkRenderer, RenderBackend, TextConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::Precision;
use crate::mark::text::MarkText;
use crate::error::ChartonError;
use crate::visual::color::SingleColor;
use polars::prelude::*;

// ============================================================================
// MARK RENDERING (Text implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkText> {
    /// Renders text marks by mapping data points to text elements in the backend.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;

        // Early return if no data to process.
        if df_source.df.height() == 0 {
            return Ok(());
        }

        // --- STEP 1: ENCODING VALIDATION ---
        let x_enc = self.encoding.x.as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding missing for Text mark".to_string()))?;
        let y_enc = self.encoding.y.as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y-axis encoding missing for Text mark".to_string()))?;
        
        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkText configuration missing".to_string()))?;

        // --- STEP 2: POSITION NORMALIZATION (Vectorized) ---
        let x_series = df_source.column(&x_enc.field)?;
        let y_series = df_source.column(&y_enc.field)?;

        let x_scale_trait = context.coord.get_x_scale();
        let y_scale_trait = context.coord.get_y_scale();

        let x_norms = x_scale_trait.scale_type().normalize_series(x_scale_trait, &x_series)?;
        let y_norms = y_scale_trait.scale_type().normalize_series(y_scale_trait, &y_series)?;

        // --- STEP 3: TEXT CONTENT RESOLUTION ---
        // Dynamically pull text from the data column if encoded, otherwise use static text.
        let text_values: Vec<String> = if let Some(ref text_enc) = self.encoding.text {
            let s = df_source.column(&text_enc.field)?;
            s.cast(&DataType::String)?
                .str()?
                .into_iter()
                .map(|opt_s| opt_s.unwrap_or("").to_string())
                .collect()
        } else {
            vec![mark_config.text.clone(); df_source.df.height()]
        };

        // --- STEP 4: COLOR MAPPING (Aesthetics) ---
        let color_iter: Box<dyn Iterator<Item = SingleColor>> = if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            let norms = s_trait.scale_type().normalize_series(s_trait, &s)?;
            let l_max = s_trait.logical_max();
            
            let color_vec: Vec<SingleColor> = norms.into_iter()
                .map(|opt_n| {
                    s_trait.mapper()
                        .map(|m| m.map_to_color(opt_n.unwrap_or(0.0), l_max))
                        .unwrap_or_else(|| SingleColor::from("black"))
                })
                .collect();
            Box::new(color_vec.into_iter())
        } else {
            Box::new(std::iter::repeat(mark_config.color.clone()))
        };

        // --- STEP 5: GEOMETRY PROJECTION & EMIT ---
        for ((x_n, y_n), (content, fill_color)) in x_norms.into_iter()
            .zip(y_norms.into_iter())
            .zip(text_values.into_iter().zip(color_iter))
        {
            let x_norm = x_n.unwrap_or(0.0);
            let y_norm = y_n.unwrap_or(0.0);
            
            // Transform normalized [0, 1] units to physical panel pixels.
            let (px, py) = context.transform(x_norm, y_norm);

            let text_config = TextConfig {
                x: px as Precision,
                y: py as Precision,
                text: content,
                font_size: mark_config.font_size as Precision,
                font_family: mark_config.font_family.clone(),
                color: fill_color,
                text_anchor: mark_config.text_anchor.to_string(), // Via our Display trait impl
                font_weight: mark_config.font_weight.to_string(), // Via our Display trait impl
                opacity: mark_config.opacity as Precision,
            };

            backend.draw_text(text_config);
        }

        Ok(())
    }
}