use crate::chart::Chart;
use crate::mark::text::MarkText;

/// Extension implementation for `Chart` to support Text Labels (MarkText).
impl Chart<MarkText> {
    /// Configures the visual properties of the text mark (font size, anchor, content).
    pub fn configure_text<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkText) -> MarkText,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
