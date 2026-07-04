use crate::chart::Chart;
use crate::mark::geo_path::MarkGeoPath;

/// Extension implementation for `Chart` to support Geographic Path Charts (MarkGeoPath).
impl Chart<MarkGeoPath> {
    /// Configures the visual properties of the geographic mark using a closure.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Chart::build(ds)?
    ///     .mark_geoshape()?
    ///     .encode((alt::x("lon"), alt::y("lat"), alt::path_group("region")))?
    ///     .configure_geoshape(|m| m
    ///         .with_fill("steelblue")
    ///         .with_stroke("white")
    ///         .with_stroke_width(0.5)
    ///     )?
    ///     .save("map.svg")?;
    /// ```
    pub fn configure_geoshape<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkGeoPath) -> MarkGeoPath,
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}
