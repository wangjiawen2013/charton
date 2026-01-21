use crate::chart::Chart;
use crate::mark::area::MarkArea;

/// Extension implementation for `Chart` to support Area Charts (MarkArea).
impl Chart<MarkArea> {
    /// Initializes a new `MarkArea` layer.
    pub fn mark_area(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkArea::default());
        }
        self
    }

    /// Configures the visual properties of the area mark using a closure.
    pub fn configure_area<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkArea) -> MarkArea 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}