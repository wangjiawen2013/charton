use crate::chart::Chart;
use crate::mark::rect::MarkRect;
use crate::core::context::PanelContext;
use crate::visual::color::SingleColor;
use crate::error::ChartonError;
use polars::prelude::*;

/// Extension implementation for `Chart` to support Heatmaps/Rectangles (MarkRect).
impl Chart<MarkRect> {
    /// Initializes a new `MarkRect` layer.
    pub fn mark_rect(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkRect::default());
        }
        self
    }

    /// Configures the visual properties of the rectangle mark using a closure.
    pub fn configure_rect<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkRect) -> MarkRect 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }

    /// Calculates the pixel dimensions for a single rectangle tile.
    /// It uses the 'bins' hint resolved during the encoding phase to ensure 
    /// visual consistency with the coordinate axes.
    pub(crate) fn calculate_rect_size(&self, context: &PanelContext) -> (f64, f64) {
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
    pub(crate) fn resolve_rect_colors(
        &self, 
        df: &DataFrame, 
        context: &PanelContext, 
        fallback: &SingleColor
    ) -> Result<Box<dyn Iterator<Item = SingleColor>>, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            
            let norms = s_trait.scale_type().normalize_series(s_trait, s)?;
            let l_max = s_trait.logical_max();

            let colors: Vec<SingleColor> = norms.into_iter()
                .map(|opt_n| {
                    s_trait.mapper()
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