//! Renderer for the violin mark.
//!
//! Draws the `x`/`y`/`path_group` vertices as closed polygons via the shared
//! polygon renderer, with the violin's fill, stroke and opacity.

use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RenderBackend};
use crate::error::ChartonError;
use crate::mark::geo_path::MarkGeoPath;
use crate::mark::violin::MarkViolin;

impl MarkRenderer for Chart<MarkViolin> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Mark("MarkViolin configuration is missing".to_string())
        })?;

        let polygon = MarkGeoPath::new()
            .with_closed(true)
            .with_fill(mark.color)
            .with_opacity(mark.opacity)
            .with_stroke(mark.stroke)
            .with_stroke_width(mark.stroke_width);

        // The data and encoding already describe the polygon; only the mark
        // configuration has to be swapped for the shared renderer.
        let as_polygon = Chart::<MarkGeoPath> {
            data: self.data.clone(),
            encoding: self.encoding.clone(),
            mark: Some(polygon),
            source_data: None,
        };

        as_polygon.render_marks(backend, context)
    }
}
