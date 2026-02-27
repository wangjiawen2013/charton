use crate::chart::Chart;
use crate::mark::rule::MarkRule;

/// Extension implementation for `Chart` to support Rule lines (MarkRule).
impl Chart<MarkRule> {
    /// Configures the visual properties of the rule mark (thresholds, guides).
    pub fn configure_rule<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkRule) -> MarkRule,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
