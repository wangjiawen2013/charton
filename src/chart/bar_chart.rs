use crate::chart::Chart;
use crate::mark::bar::MarkBar;

/// Extension implementation for `Chart` to support Bar Charts (MarkBar).
impl Chart<MarkBar> {
    /// Initializes a new `MarkBar` layer.
    pub fn mark_bar(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkBar::default());
        }
        self
    }

    /// Configures the visual properties of the bar mark using a closure.
    /// 
    /// # Example
    /// ```
    /// chart.mark_bar()
    ///      .configure_bar(|b| b.color("steelblue").width(0.6).span(0.8))
    /// ```
    pub fn configure_bar<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkBar) -> MarkBar 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}