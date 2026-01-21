/// Represents different geometric shapes for data points.
/// Supports conversion from strings for a fluent API and provides 
/// integer IDs for GPU-accelerated rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)] // Added Copy/Eq for easier usage
pub enum PointShape {
    Circle = 0,
    Square = 1,
    Triangle = 2,
    Star = 3,
    Diamond = 4,
    Pentagon = 5,
    Hexagon = 6,
    Octagon = 7,
}

impl PointShape {
    /// Returns a unique integer ID for the shape. 
    /// This is crucial for passing shape data to GPU shaders (wgpu).
    pub fn gpu_id(&self) -> u32 {
        *self as u32
    }

    /// Shapes used for default mapping in legends.
    pub(crate) const LEGEND_SHAPES: &'static [PointShape] = &[
        PointShape::Circle,
        PointShape::Square,
        PointShape::Triangle,
        PointShape::Star,
        PointShape::Diamond,
        PointShape::Pentagon,
        PointShape::Hexagon,
        PointShape::Octagon,
    ];
}

// --- Fluent API Support: From string literals ---

impl From<&str> for PointShape {
    /// Converts a string like "circle" or "square" into a PointShape.
    /// Defaults to Circle if the string is unrecognized.
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "circle" => PointShape::Circle,
            "square" => PointShape::Square,
            "triangle" => PointShape::Triangle,
            "star" => PointShape::Star,
            "diamond" => PointShape::Diamond,
            "pentagon" => PointShape::Pentagon,
            "hexagon" => PointShape::Hexagon,
            "octagon" => PointShape::Octagon,
            _ => PointShape::Circle, // Robust fallback
        }
    }
}

impl From<String> for PointShape {
    fn from(s: String) -> Self {
        PointShape::from(s.as_str())
    }
}

impl Default for PointShape {
    fn default() -> Self {
        PointShape::Circle
    }
}