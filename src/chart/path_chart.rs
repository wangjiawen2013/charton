use crate::chart::Chart;
use crate::mark::path::MarkPath;

/// Extension implementation for `Chart` to support open path charts
/// ([`MarkPath`]).
impl Chart<MarkPath> {
    /// Configures the visual properties of the path mark using a closure.
    ///
    /// # Example
    /// ```rust,ignore
    /// chart.mark_path()
    ///      .configure_path(|p| p.with_stroke("#2563eb").with_stroke_width(1.5))
    /// ```
    pub fn configure_path<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkPath) -> MarkPath,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
