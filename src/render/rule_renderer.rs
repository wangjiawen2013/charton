use crate::core::layer::{MarkRenderer, RenderBackend, LineConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::mark::rule::MarkRule;
use crate::error::ChartonError;
use crate::visual::color::SingleColor;
use crate::Precision;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkRule> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df_source = &self.data;
        let row_count = df_source.df.height();
        if row_count == 0 { return Ok(()); }

        // --- STEP 1: VALIDATION ---
        let x_enc = self.encoding.x.as_ref()
            .ok_or_else(|| ChartonError::Encoding("X-axis encoding missing".into()))?;
        let y_enc = self.encoding.y.as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y-axis encoding missing".into()))?;
        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkRule config missing".into()))?;

        // --- STEP 2: VECTORIZED COORDINATE UNIFICATION ---
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();
        let x_series = df_source.column(&x_enc.field)?;
        let y_series = df_source.column(&y_enc.field)?;

        let x_norms = x_scale.scale_type().normalize_series(x_scale, &x_series)?;
        
        let (y1_norms, y2_norms) = if let Some(y2_enc) = &self.encoding.y2 {
            let y1 = y_scale.scale_type().normalize_series(y_scale, &y_series)?;
            let y2_series = df_source.column(&y2_enc.field)?;
            let y2 = y_scale.scale_type().normalize_series(y_scale, &y2_series)?;
            (y1, y2)
        } else {
            let y_max = y_scale.logical_max();
            (Float64Chunked::full("y1".into(), 0.0, row_count), 
             Float64Chunked::full("y2".into(), y_max, row_count))
        };

        // --- STEP 3: COLOR MAPPING ---
        let color_iter: Box<dyn Iterator<Item = SingleColor>> = if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df_source.column(&mapping.field)?;
            let s_trait = mapping.scale_impl.as_ref();
            let norms = s_trait.scale_type().normalize_series(s_trait, &s)?;
            let l_max = s_trait.logical_max();
            let mapper = s_trait.mapper();
            
            let color_vec: Vec<SingleColor> = norms.into_iter()
                .map(|opt_n| {
                    mapper.map(|m| m.map_to_color(opt_n.unwrap(), l_max))
                        .unwrap_or_else(|| SingleColor::from("#333333"))
                }).collect();
            Box::new(color_vec.into_iter())
        } else {
            Box::new(std::iter::repeat(mark_config.color.clone()))
        };

        // --- STEP 4: UNIFIED RENDERING LOOP ---
        let stroke_width = mark_config.stroke_width as Precision;
        let is_flipped = context.coord.is_flipped();

        for (((x_n_opt, y1_n_opt), y2_n_opt), mapped_color) in x_norms.into_iter()
            .zip(y1_norms.into_iter())
            .zip(y2_norms.into_iter())
            .zip(color_iter)
        {
            let x_n = x_n_opt.unwrap();
            let y1_n = y1_n_opt.unwrap();
            let y2_n = y2_n_opt.unwrap();

            // Project endpoints to physical pixels
            let (p1_x, p1_y) = context.transform(x_n, y1_n);
            let (p2_x, p2_y) = context.transform(x_n, y2_n);

            // Correct orientation based on coord_flip
            let (final_x1, final_y1, final_x2, final_y2) = if !is_flipped {
                // Standard: Rule is vertical (Y changes, X is constant)
                (p1_x, p1_y, p1_x, p2_y)
            } else {
                // Flipped: Rule is horizontal (X changes, Y is constant)
                (p1_x, p1_y, p2_x, p1_y)
            };

            backend.draw_line(LineConfig {
                x1: final_x1 as Precision,
                y1: final_y1 as Precision,
                x2: final_x2 as Precision,
                y2: final_y2 as Precision,
                color: mapped_color,
                width: stroke_width,
                opacity: mark_config.opacity as Precision,
                dash: None,
            });
        }

        Ok(())
    }
}