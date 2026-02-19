use crate::chart::Chart;
use crate::mark::histogram::MarkHist;

/// Extension implementation for `Chart` to support Histograms (MarkHist).
impl Chart<MarkHist> {
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