use crate::chart::Chart;
use crate::mark::point::MarkPoint;

/// Extension implementation for `Chart` to support Scatter Plots (MarkPoint).
/// 
/// This module provides the user-facing API to initialize and configure 
/// the visual properties of point marks.
impl Chart<MarkPoint> {
    
    /// Initializes a new `MarkPoint` (Scatter Plot) layer.
    /// 
    /// If a mark configuration already exists in this layer, it is preserved; 
    /// otherwise, a new `MarkPoint` with default settings is created. 
    /// This ensures that `mark_point()` can be called safely at any point 
    /// in the method chain.
    pub fn mark_point(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkPoint::default());
        }
        self
    }

    /// Configures the visual properties of the point mark using a closure.
    /// 
    /// This is the primary entry point for customizing the look of the scatter plot.
    /// Since [MarkPoint] implements a fluent builder interface, you can chain 
    /// multiple property changes inside the closure efficiently.
    ///
    /// # Example
    /// ```
    /// chart.mark_point()
    ///      .configure_point(|m| m.color("red").size(5.0).opacity(0.8))
    /// ```
    pub fn configure_point<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkPoint) -> MarkPoint 
    {
        // Extract the existing mark or start with a default one
        let mark = self.mark.take().unwrap_or_default();
        
        // Apply the configuration closure and re-insert the mark
        self.mark = Some(f(mark));
        self
    }
}