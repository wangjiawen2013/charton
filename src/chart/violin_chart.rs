use crate::chart::Chart;
use crate::mark::violin::MarkViolin;

/// Extension implementation for `Chart` to support violins (`MarkViolin`).
impl Chart<MarkViolin> {
    /// Configures the violin recipe and its visual style.
    ///
    /// ```rust,ignore
    /// chart!(penguins)?
    ///     .mark_violin()?
    ///     .configure_violin(|v| v.with_split(true).with_color("#95a5a6"))
    ///     .encode((alt::x("Species"), alt::y("Body Mass (g)"), alt::color("Sex")))?
    /// ```
    pub fn configure_violin<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkViolin) -> MarkViolin,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
