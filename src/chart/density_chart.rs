use crate::chart::Chart;
use crate::mark::density::MarkDensity;

/// Extension implementation for `Chart` to support density curves (`MarkDensity`).
impl Chart<MarkDensity> {
    /// Configures the density recipe and its visual style.
    ///
    /// ```rust,ignore
    /// chart!(iris)?
    ///     .mark_density()?
    ///     .configure_density(|d| d.with_opacity(0.5).with_bandwidth(BandwidthType::Silverman))
    ///     .encode((alt::x("sepal_length"), alt::color("species")))?
    /// ```
    pub fn configure_density<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkDensity) -> MarkDensity,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
