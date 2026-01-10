use crate::visual::shape::PointShape;
use crate::visual::color::{ColorMap, ColorPalette};

/// Defines how data values (after normalization) are mapped to visual properties.
/// 
/// This enum supports different types of visual encodings, including continuous
/// color gradients, discrete color palettes, geometric shapes, and point sizes.
pub enum VisualMapper {
    /// Continuous color mapping for numerical data.
    ContinuousColor {
        map: ColorMap,
    },
    /// Discrete color mapping for categorical data.
    DiscreteColor {
        palette: ColorPalette,
    },
    /// Geometric shape mapping for categorical data.
    Shape {
        /// Optional list of shapes. If None, defaults to `PointShape::LEGEND_SHAPES`.
        custom_shapes: Option<Vec<PointShape>>,
    },
    /// Size mapping for numerical data (linear interpolation).
    Size {
        /// Tuple representing (min_size, max_size).
        range: (f64, f64),
    },
}

impl VisualMapper {
    /// Maps a normalized value [0.0, 1.0] to a hex color string.
    /// 
    /// # Arguments
    /// * `norm` - The normalized value from the scale (usually 0.0 to 1.0).
    /// * `domain_max` - The maximum index of the domain (used for discrete mapping).
    pub fn map_to_color(&self, norm: f64, domain_max: f64) -> String {
        match self {
            VisualMapper::ContinuousColor { map } => map.get_color(norm),
            VisualMapper::DiscreteColor { palette } => {
                // Calculate index based on normalization and domain size
                let index = (norm * domain_max).round() as usize;
                palette.get_color(index)
            }
            _ => "#000000".to_string(), // Default fallback color
        }
    }

    /// Maps a normalized value to a `PointShape`.
    /// 
    /// # Arguments
    /// * `norm` - The normalized value from the scale.
    /// * `domain_max` - The maximum index of the domain (number of categories - 1).
    pub fn map_to_shape(&self, norm: f64, domain_max: f64) -> PointShape {
        match self {
            VisualMapper::Shape { custom_shapes } => {
                // Use custom list if provided, otherwise fallback to the built-in legend shapes
                let shapes = custom_shapes
                    .as_deref()
                    .unwrap_or(PointShape::LEGEND_SHAPES);
                
                if shapes.is_empty() { 
                    return PointShape::Circle; 
                }

                let index = (norm * domain_max).round() as usize;
                shapes[index % shapes.len()].clone()
            }
            _ => PointShape::Circle, // Default fallback shape
        }
    }

    /// Maps a normalized value to a physical size (radius or width).
    /// 
    /// # Arguments
    /// * `norm` - The normalized value (0.0 for min_size, 1.0 for max_size).
    pub fn map_to_size(&self, norm: f64) -> f64 {
        match self {
            VisualMapper::Size { range } => {
                range.0 + norm * (range.1 - range.0)
            }
            _ => 5.0, // Default fallback size
        }
    }
}