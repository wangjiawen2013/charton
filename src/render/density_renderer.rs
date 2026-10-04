//! Renderer for the density mark.
//!
//! Draws the `(value, density)` table as a filled area via the shared area
//! renderer.

use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RenderBackend};
use crate::error::ChartonError;
use crate::mark::area::MarkArea;
use crate::mark::density::MarkDensity;

impl MarkRenderer for Chart<MarkDensity> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Mark("MarkDensity configuration is missing".to_string())
        })?;

        let area = MarkArea::new()
            .with_color(mark.color)
            .with_opacity(mark.opacity)
            .with_stroke(mark.stroke)
            .with_stroke_width(mark.stroke_width)
            .with_dash(mark.dash.clone());

        let as_area = Chart::<MarkArea> {
            data: self.data.clone(),
            encoding: self.encoding.clone(),
            mark: Some(area),
            source_data: None,
        };

        as_area.render_marks(backend, context)
    }
}
