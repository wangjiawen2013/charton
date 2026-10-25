use crate::chart::Chart;
use crate::mark::polygon::MarkPolygon;

/// Extension implementation for `Chart` to support closed, filled polygon
/// charts ([`MarkPolygon`]).
impl Chart<MarkPolygon> {
    /// Configures the visual properties of the polygon mark using a closure.
    ///
    /// # Example
    /// ```rust,ignore
    /// chart.mark_polygon()
    ///      .configure_polygon(|p| p.with_fill("#dbeafe").with_stroke("none"))
    /// ```
    pub fn configure_polygon<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkPolygon) -> MarkPolygon,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
