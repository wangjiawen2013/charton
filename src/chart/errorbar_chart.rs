use crate::chart::Chart;
use crate::mark::errorbar::MarkErrorBar;

/// Extension implementation for `Chart` to support Error Bar plots.
///
/// Error bars are used to visualize statistical uncertainty, showing
/// confidence intervals or standard deviations around a central value.
impl Chart<MarkErrorBar> {
    /// Configures the visual properties of the error bar mark using a closure.
    ///
    /// This allows for detailed customization of the error bar's appearance,
    /// such as changing the cap length, stroke thickness, or toggling the
    /// visibility of the center point.
    ///
    /// # Example
    /// ```rust,ignore
    /// chart.mark_errorbar()
    ///      .configure_errorbar(|m| m.with_color("blue").with_cap_length(5.0).with_center(true))
    /// ```
    pub fn configure_errorbar<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkErrorBar) -> MarkErrorBar,
    {
        // Extract the existing mark or start with a default one
        let mark = self.mark.take().unwrap_or_default();

        // Apply the configuration closure and re-insert the mark
        self.mark = Some(f(mark));
        self
    }
}
