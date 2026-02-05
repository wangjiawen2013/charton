use crate::chart::Chart;
use crate::mark::line::MarkLine;

/// Extension implementation for `Chart` to support Line Charts (MarkLine).
impl Chart<MarkLine> {
    /// Initializes a new `MarkLine` layer.
    /// 
    /// If a mark configuration already exists, it is preserved; 
    /// otherwise, a default `MarkLine` is created.
    pub fn mark_line(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkLine::default());
        }
        self
    }

    /// Configures the visual properties of the line mark using a closure.
    /// 
    /// # Example
    /// ```
    /// chart.mark_line()
    ///      .configure_line(|l| l.color("blue").stroke_width(2.5).interpolation("basis"))
    /// ```
    pub fn configure_line<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkLine) -> MarkLine 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}