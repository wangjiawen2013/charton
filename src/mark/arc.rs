use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for circular/arc geometries, used to render Pie, Donut, and Rose charts.
/// 
/// Unlike a Bar mark which exists in Cartesian space, the Arc mark is defined by 
/// angular and radial constraints.
#[derive(Debug, Clone)]
pub struct MarkArc {
    // --- Visual Aesthetics ---
    pub(crate) color: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,

    // --- Angular Geometry ---
    /// The proportion of the allocated angular slot to fill (0.0 to 1.0).
    /// Used primarily in Rose charts to create gaps between categories.
    pub(crate) width: f64, 
    
    /// Fixed angular gap between sectors, measured in radians.
    /// Helpful for creating "exploded" or clean-separated Pie charts.
    pub(crate) pad_angle: f64,

    // --- Radial Geometry ---
    /// The inner radius ratio (0.0 to 1.0). 
    /// 0.0 creates a standard Pie chart; >0.0 creates a Donut chart.
    pub(crate) inner_radius: f64, 
    
    /// The outer radius ratio (usually 1.0). 
    /// Can be used to shrink the entire chart relative to the container.
    pub(crate) outer_radius: f64,

    /// Experimental: Rounding of the arc corners. 
    /// Measured in pixels relative to the final render.
    pub(crate) corner_radius: f64,
}

impl MarkArc {
    /// Creates a new Arc mark with default Pie chart settings.
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("steelblue"),
            opacity: 1.0,
            stroke: SingleColor::new("white"), 
            stroke_width: 0.0,
            
            width: 1.0,        // Fill the whole angular slot by default
            pad_angle: 0.0,    // No gaps between slices
            
            inner_radius: 0.0, // Solid center (Pie)
            outer_radius: 1.0, // Full extent
            corner_radius: 0.0, // Sharp edges
        }
    }

    // --- Fluent Builders ---

    /// Sets the inner radius (e.g., 0.5 for a Donut chart).
    /// Clamped between 0.0 and 1.0.
    pub fn with_inner_radius(mut self, radius: f64) -> Self {
        self.inner_radius = radius.clamp(0.0, 1.0);
        self
    }

    /// Sets the outer radius (usually 1.0).
    pub fn with_outer_radius(mut self, radius: f64) -> Self {
        self.outer_radius = radius.clamp(0.0, 1.0);
        self
    }

    /// Sets the outer radius.
    pub fn with_corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius.clamp(0.0, 1.0);
        self
    }


    /// Sets the angular width proportion (0.0 to 1.0).
    /// Use this to make Rose chart "petals" thinner.
    pub fn with_width(mut self, width: f64) -> Self {
        self.width = width.clamp(0.0, 1.0);
        self
    }

    /// Sets a fixed gap (in radians) between sectors.
    pub fn with_pad_angle(mut self, radians: f64) -> Self {
        self.pad_angle = radians;
        self
    }

    /// Sets the color of the sectors.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the sectors.
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke (border) color and width.
    pub fn with_stroke(mut self, color: impl Into<SingleColor>, width: f64) -> Self {
        self.stroke = color.into();
        self.stroke_width = width;
        self
    }
}

impl Mark for MarkArc {
    /// Returns the unique identifier for this mark. 
    /// Used by the engine to select the correct validation and rendering pipelines.
    fn mark_type(&self) -> &'static str {
        "arc" 
    }
}

impl Default for MarkArc {
    fn default() -> Self { Self::new() }
}
