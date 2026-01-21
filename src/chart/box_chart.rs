use crate::chart::Chart;
use crate::mark::boxplot::MarkBoxplot;

/// Extension implementation for `Chart` to support Box Plots (MarkBoxplot).
impl Chart<MarkBoxplot> {
    /// Initializes a new `MarkBoxplot` layer.
    pub fn mark_boxplot(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkBoxplot::default());
        }
        self
    }

    /// Configures boxplot properties like outliers and spacing.
    pub fn configure_boxplot<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkBoxplot) -> MarkBoxplot 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}