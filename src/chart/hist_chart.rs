use crate::chart::Chart;
use crate::mark::histogram::MarkHist;

/// Extension implementation for `Chart` to support Histograms (MarkHist).
impl Chart<MarkHist> {
    /// Initializes a new `MarkHist` layer.
    pub fn mark_hist(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkHist::default());
        }
        self
    }

    /// Configures the visual properties of the histogram bars using a closure.
    pub fn configure_hist<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkHist) -> MarkHist 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}