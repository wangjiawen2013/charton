use crate::chart::Chart;
use crate::mark::contour::MarkContour;

/// Extension implementation for `Chart` to support contours (`MarkContour`).
impl Chart<MarkContour> {
    /// Configures the contour recipe and its visual style.
    ///
    /// ```rust,ignore
    /// chart!(x, y, z)?
    ///     .mark_contour("z")?
    ///     .configure_contour(|c| c.with_levels(12).with_stroke_width(1.5))
    ///     .encode((alt::x("x"), alt::y("y")))?
    /// ```
    pub fn configure_contour<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkContour) -> MarkContour,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
