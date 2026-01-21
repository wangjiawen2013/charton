use crate::chart::Chart;
use crate::mark::rule::MarkRule;

/// Extension implementation for `Chart` to support Rule lines (MarkRule).
impl Chart<MarkRule> {
    /// Initializes a new `MarkRule` layer.
    pub fn mark_rule(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkRule::default());
        }
        self
    }

    /// Configures the visual properties of the rule mark (thresholds, guides).
    pub fn configure_rule<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkRule) -> MarkRule 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}