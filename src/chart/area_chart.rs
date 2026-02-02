use crate::chart::Chart;
use crate::mark::area::MarkArea;
use crate::core::context::PanelContext;
use crate::visual::color::SingleColor;
use crate::error::ChartonError;
use polars::prelude::*;

/// Extension implementation for `Chart` to support Area Charts (MarkArea).
impl Chart<MarkArea> {
    /// Initializes a new `MarkArea` layer.
    pub fn mark_area(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkArea::default());
        }
        self
    }

    /// Configures the visual properties of the area mark using a closure.
    pub fn configure_area<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkArea) -> MarkArea 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }

    /// Resolves a single fill color for the entire area group.
    pub(crate) fn resolve_group_color(&self, df: &DataFrame, context: &PanelContext, fallback: &SingleColor) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            
            // Map the first value of the group to a color.
            let first_val_norm = s_trait.scale_type().normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = first_val_norm.get(0).unwrap_or(0.0);
            
            Ok(s_trait.mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| fallback.clone()))
        } else {
            Ok(fallback.clone())
        }
    }
}