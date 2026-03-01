use crate::chart::Chart;
use crate::mark::line::MarkLine;

/// Extension implementation for `Chart` to support Line Charts (MarkLine).
impl Chart<MarkLine> {
    /// Configures the visual properties of the line mark using a closure.
    ///
    /// # Example
    /// ```rust,ignore
    /// chart.mark_line()
    ///      .configure_line(|l| l.color("blue").stroke_width(2.5).interpolation("basis"))
    /// ```
    pub fn configure_line<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkLine) -> MarkLine,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
