use crate::scale::ScaleTrait;
use crate::scale::mapper::VisualMapper;

/// `GlobalAesthetics` provides a unified set of visual mapping rules for the entire chart.
/// 
/// Instead of storing pre-calculated values for every data point, it stores the 
/// scales and mappers (the "rules"). This ensures that all layers in a multi-layer 
/// chart use the same visual language (e.g., the same color means the same value).
pub struct GlobalAesthetics {
    /// Color: (Scale for math, Mapper for vision)
    pub color: Option<(Box<dyn ScaleTrait>, VisualMapper)>,
    
    /// Shape: (Scale for math, Mapper for vision)
    pub shape: Option<(Box<dyn ScaleTrait>, VisualMapper)>,
    
    /// Size: (Scale for math, Mapper for vision)
    pub size: Option<(Box<dyn ScaleTrait>, VisualMapper)>,
}

impl GlobalAesthetics {
    /// Constructs a new `GlobalAesthetics` with resolved rules.
    /// 
    /// This is typically called by `resolve_rendering_layout` after aggregating 
    /// domains from all chart layers.
    pub fn new(
        color: Option<(Box<dyn ScaleTrait>, VisualMapper)>,
        shape: Option<(Box<dyn ScaleTrait>, VisualMapper)>,
        size: Option<(Box<dyn ScaleTrait>, VisualMapper)>,
    ) -> Self {
        Self {
            color,
            shape,
            size,
        }
    }
}