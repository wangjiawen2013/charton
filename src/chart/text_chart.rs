use crate::chart::Chart;
use crate::mark::text::MarkText;

/// Extension implementation for `Chart` to support Text Labels (MarkText).
impl Chart<MarkText> {
    /// Initializes a new `MarkText` layer.
    pub fn mark_text(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkText::default());
        }
        self
    }

    /// Configures the visual properties of the text mark (font size, anchor, content).
    pub fn configure_text<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkText) -> MarkText 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}