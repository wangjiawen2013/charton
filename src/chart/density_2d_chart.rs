use crate::chart::Chart;
use crate::mark::density_2d::MarkDensity2D;

/// Extension implementation for `Chart` to support 2-D density heatmaps
/// (`MarkDensity2D`).
impl Chart<MarkDensity2D> {
    /// Configures the 2-D density recipe and its visual style.
    ///
    /// ```rust,ignore
    /// chart!(iris)?
    ///     .mark_density_2d()?
    ///     .configure_density_2d(|d| d.with_grid_size(60).with_padding(0.2))
    ///     .encode((alt::x("sepal_length"), alt::y("petal_length")))?
    /// ```
    pub fn configure_density_2d<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkDensity2D) -> MarkDensity2D,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
