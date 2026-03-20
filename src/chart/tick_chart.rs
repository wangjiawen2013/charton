use crate::chart::Chart;
use crate::mark::tick::MarkTick;

/// Extension implementation for `Chart` to support Tick Charts.
///
/// This module provides the user-facing API to initialize and configure
/// the visual properties of tick marks.
impl Chart<MarkTick> {
    /// Configures the visual properties of the tick mark using a closure.
    ///
    /// This is the primary entry point for customizing the look of the tick chart.
    /// Since [MarkTick] implements a fluent builder interface, you can chain
    /// multiple property changes inside the closure efficiently.
    pub fn configure_tick<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MarkTick) -> MarkTick,
    {
        // Extract the existing mark or start with a default one
        let mark = self.mark.take().unwrap_or_default();

        // Apply the configuration closure and re-insert the mark
        self.mark = Some(f(mark));
        self
    }
}
