//! Renderer for the 2-D density mark.
//!
//! Draws the heatmap cells via the shared rectangle renderer.

use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RenderBackend};
use crate::error::ChartonError;
use crate::mark::density_2d::MarkDensity2D;
use crate::mark::rect::MarkRect;

impl MarkRenderer for Chart<MarkDensity2D> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Mark("MarkDensity2D configuration is missing".to_string())
        })?;

        let rect = MarkRect::new()
            .with_color(mark.color)
            .with_opacity(mark.opacity)
            .with_stroke(mark.stroke)
            .with_stroke_width(mark.stroke_width);

        let as_rect = Chart::<MarkRect> {
            data: self.data.clone(),
            encoding: self.encoding.clone(),
            mark: Some(rect),
            source_data: None,
        };

        as_rect.render_marks(backend, context)
    }
}
