use crate::chart::Chart;
use crate::mark::line::MarkLine;
use crate::core::context::PanelContext;
use crate::visual::color::SingleColor;
use crate::error::ChartonError;
use polars::prelude::*;

/// Extension implementation for `Chart` to support Line Charts (MarkLine).
impl Chart<MarkLine> {
    /// Initializes a new `MarkLine` layer.
    /// 
    /// If a mark configuration already exists, it is preserved; 
    /// otherwise, a default `MarkLine` is created.
    pub fn mark_line(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkLine::default());
        }
        self
    }

    /// Configures the visual properties of the line mark using a closure.
    /// 
    /// # Example
    /// ```
    /// chart.mark_line()
    ///      .configure_line(|l| l.color("blue").stroke_width(2.5).interpolation("basis"))
    /// ```
    pub fn configure_line<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkLine) -> MarkLine 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }

    /// Injects corner points for Step-After interpolation.
    pub(crate) fn expand_step_after(&self, points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        let mut expanded = Vec::with_capacity(points.len() * 2);
        for i in 0..points.len() - 1 {
            let (x1, y1) = points[i];
            let (x2, _y2) = points[i+1];
            expanded.push((x1, y1));
            expanded.push((x2, y1)); // The "Step"
        }
        expanded.push(*points.last().unwrap());
        expanded
    }

    /// Injects corner points for Step-Before interpolation.
    pub(crate) fn expand_step_before(&self, points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        let mut expanded = Vec::with_capacity(points.len() * 2);
        for i in 0..points.len() - 1 {
            let (x1, y1) = points[i];
            let (_x2, y2) = points[i+1];
            expanded.push((x1, y1));
            expanded.push((x1, y2)); // The "Step"
        }
        expanded.push(*points.last().unwrap());
        expanded
    }

    /// Resolves the color for a specific group of data.
    pub(crate) fn resolve_group_color(&self, df: &DataFrame, context: &PanelContext, fallback: &SingleColor) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            // Since all points in a group share the same category, we just map the first value
            let first_val = s_trait.scale_type().normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = first_val.get(0).unwrap_or(0.0);
            Ok(s_trait.mapper().map(|m| m.map_to_color(norm, s_trait.logical_max())).unwrap_or_else(|| fallback.clone()))
        } else {
            Ok(fallback.clone())
        }
    }
}