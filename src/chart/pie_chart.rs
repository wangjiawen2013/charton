use crate::chart::Chart;
use crate::mark::arc::MarkArc;

/// Extension implementation for `Chart` to support Pie/Donut Charts (MarkArc).
impl Chart<MarkArc> {
    /// Initializes a new `MarkArc` layer.
    pub fn mark_arc(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkArc::default());
        }
        self
    }

    /// Configures the visual properties of the arc mark (e.g., inner radius for donuts).
    pub fn configure_arc<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkArc) -> MarkArc 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}