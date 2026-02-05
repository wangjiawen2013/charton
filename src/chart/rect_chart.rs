use crate::chart::Chart;
use crate::mark::rect::MarkRect;

/// Extension implementation for `Chart` to support Heatmaps/Rectangles (MarkRect).
impl Chart<MarkRect> {
    /// Initializes a new `MarkRect` layer.
    pub fn mark_rect(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkRect::default());
        }
        self
    }

    /// Configures the visual properties of the rectangle mark using a closure.
    pub fn configure_rect<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkRect) -> MarkRect 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}