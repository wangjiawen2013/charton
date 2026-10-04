//! Renderer for the contour mark.
//!
//! Draws the `x`/`y`/`path_group`/`level` table as open stroked polylines via
//! the shared path renderer.

use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RenderBackend};
use crate::error::ChartonError;
use crate::mark::contour::MarkContour;
use crate::mark::geo_path::MarkGeoPath;

impl MarkRenderer for Chart<MarkContour> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Mark("MarkContour configuration is missing".to_string())
        })?;

        // An iso-line is an open path: stroked, never closed and never filled.
        let path = MarkGeoPath::new()
            .with_closed(false)
            .with_fill("none")
            .with_stroke(mark.stroke)
            .with_stroke_width(mark.stroke_width)
            .with_opacity(mark.opacity)
            .with_dash(mark.dash.clone());

        let as_path = Chart::<MarkGeoPath> {
            data: self.data.clone(),
            encoding: self.encoding.clone(),
            mark: Some(path),
            source_data: None,
        };

        as_path.render_marks(backend, context)
    }
}
