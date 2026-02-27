use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RectConfig, RenderBackend};
use crate::error::ChartonError;
use crate::mark::rect::MarkRect;
use crate::visual::color::SingleColor;
use polars::prelude::DataFrame;

impl MarkRenderer for Chart<MarkRect> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df = &self.data.df;
        if df.height() == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkRect configuration is missing".into()))?;

        // --- STEP 1: POSITIONING ---
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

        let x_series = df.column(&x_enc.field)?.as_materialized_series();
        let y_series = df.column(&y_enc.field)?.as_materialized_series();

        // Standardize data to [0.0, 1.0] normalized space
        let x_norms = x_scale.scale_type().normalize_series(x_scale, x_series)?;
        let y_norms = y_scale.scale_type().normalize_series(y_scale, y_series)?;

        // --- STEP 2: SIZE CALCULATION ---
        // We now calculate sizes based on the pre-resolved 'bins' count.
        // This ensures the rectangles fill the exact width/height allocated by the scale.
        let (rect_width, rect_height) = self.calculate_rect_size(context);

        // --- STEP 3: COLOR MAPPING ---
        let color_iter = self.resolve_rect_colors(df, context, &mark_config.color)?;

        // --- STEP 4: RENDERING LOOP ---
        // Iterate through normalized coordinates and draw centered rectangles
        for ((opt_x, opt_y), fill_color) in
            x_norms.into_iter().zip(y_norms.into_iter()).zip(color_iter)
        {
            let x_n = opt_x.unwrap_or(0.0);
            let y_n = opt_y.unwrap_or(0.0);

            // Convert normalized [0,1] to pixel coordinates
            let (px, py) = context.transform(x_n, y_n);

            backend.draw_rect(RectConfig {
                // px/py are centers; we offset by half-width/height to get the top-left corner
                x: (px - rect_width / 2.0) as Precision,
                y: (py - rect_height / 2.0) as Precision,
                width: rect_width as Precision,
                height: rect_height as Precision,
                fill: fill_color,
                stroke: mark_config.stroke.clone(),
                stroke_width: mark_config.stroke_width as Precision,
                opacity: mark_config.opacity as Precision,
            });
        }

        Ok(())
    }
}

impl Chart<MarkRect> {
    /// Calculates the pixel dimensions for a single rectangle tile.
    /// It uses the 'bins' hint resolved during the encoding phase to ensure
    /// visual consistency with the coordinate axes.
    fn calculate_rect_size(&self, context: &PanelContext) -> (f64, f64) {
        // Retrieve bin counts from encodings (resolved in apply_default_encodings)
        let x_bins = self.encoding.x.as_ref().and_then(|e| e.bins).unwrap_or(1);
        let y_bins = self.encoding.y.as_ref().and_then(|e| e.bins).unwrap_or(1);

        // Calculate the logical step size in normalized [0, 1] space.
        // If we have 10 bins, each bin occupies exactly 1/10th of the available space.
        let x_logical_step = 1.0 / (x_bins as f64);
        let y_logical_step = 1.0 / (y_bins as f64);

        // Transform the logical width into pixel width.
        // We measure the distance between the start (0,0) and the first step.
        let (p0_x, p0_y) = context.transform(0.0, 0.0);
        let (p1_x, p1_y) = context.transform(x_logical_step, y_logical_step);

        ((p1_x - p0_x).abs(), (p1_y - p0_y).abs())
    }

    /// Resolves the color stream for each rectangle, either from a mapped data
    /// column or a fallback static color.
    fn resolve_rect_colors(
        &self,
        df: &DataFrame,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> Result<Box<dyn Iterator<Item = SingleColor>>, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();

            let norms = s_trait.scale_type().normalize_series(s_trait, s)?;
            let l_max = s_trait.logical_max();

            let colors: Vec<SingleColor> = norms
                .into_iter()
                .map(|opt_n| {
                    s_trait
                        .mapper()
                        .map(|m| m.map_to_color(opt_n.unwrap_or(0.0), l_max))
                        .unwrap_or_else(|| fallback.clone())
                })
                .collect();
            Ok(Box::new(colors.into_iter()))
        } else {
            // No color mapping: return an infinite iterator of the fallback color
            Ok(Box::new(std::iter::repeat(fallback.clone())))
        }
    }
}
