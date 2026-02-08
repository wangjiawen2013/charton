use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for circular/arc geometries.
#[derive(Debug, Clone)]
pub struct MarkArc {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,
    pub(crate) width: f64,         // Angular width (ratio of category)
    pub(crate) spacing: f64,       // Angular spacing
    pub(crate) span: f64,          // Radial span (length)
    pub(crate) inner_radius: f64,  // The "hole" (0.0 for Pie, >0.0 for Donut)
}

impl MarkArc {
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("steelblue"),
            opacity: 1.0,
            stroke: SingleColor::new("white"), 
            stroke_width: 0.0,
            width: 1.0,      // Default to full angular fill
            spacing: 0.0,
            span: 1.0,       // Default to full radius
            inner_radius: 0.0, 
        }
    }

    // --- Fluent Builders ---
    pub fn with_inner_radius(mut self, radius: f64) -> Self {
        self.inner_radius = radius.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }
    
    // ... other with_ methods similar to MarkBar ...
}

impl Mark for MarkArc {
    fn mark_type(&self) -> &'static str {
        "arc" // This allows Chart::encode() to perform specific validation
    }
}

impl Default for MarkArc {
    fn default() -> Self { Self::new() }
}