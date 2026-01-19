use crate::visual::shape::PointShape;
use crate::visual::color::{ColorMap, ColorPalette};
use crate::scale::Scale;
use crate::theme::Theme;

/// Defines how data values (after normalization) are mapped to visual properties.
/// 
/// This enum supports different types of visual encodings, including continuous
/// color gradients, discrete color palettes, geometric shapes, and point sizes.
#[derive(Debug, Clone)]
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
    /// Creates a default color mapper based on whether the scale is discrete or continuous.
    pub fn new_color_default(scale_type: &Scale, theme: &Theme) -> Self {
        match scale_type {
            Scale::Discrete => {
                // Use the categorical palette from the theme
                VisualMapper::DiscreteColor {
                    palette: theme.palette.clone(),
                }
            }
            _ => {
                // Use the continuous gradient map from the theme
                VisualMapper::ContinuousColor {
                    map: theme.color_map.clone(),
                }
            }
        }
    }

    /// Creates a default size mapper with a specified physical range (e.g., 2.0 to 9.0).
    pub fn new_size_default(min: f64, max: f64) -> Self {
        VisualMapper::Size {
            range: (min, max),
        }
    }

    /// Creates a default shape mapper using the standard geometric shapes.
    pub fn new_shape_default() -> Self {
        VisualMapper::Shape {
            custom_shapes: None, // Will fallback to PointShape::LEGEND_SHAPES
        }
    }

    /// Maps a normalized value [0.0, 1.0] to a hex color string.
    /// 
    /// # Arguments
    /// * `norm` - The normalized value from the scale (usually 0.0 to 1.0).
    /// * `logical_max` - The maximum index or logical value (from scale.logical_max()).
    pub fn map_to_color(&self, norm: f64, logical_max: f64) -> String {
        match self {
            VisualMapper::ContinuousColor { map } => {
                // For continuous scales, logical_max is typically 1.0, 
                // so we map the normalized value directly.
                map.get_color(norm)
            },
            VisualMapper::DiscreteColor { palette } => {
                // For discrete scales, logical_max is (n-1).
                // We re-scale the 0-1 norm value back to the index space.
                let index = (norm * logical_max).round() as usize;
                palette.get_color(index)
            }
            _ => "#000000".to_string(), // Default fallback color
        }
    }

    /// Maps a normalized value to a `PointShape`.
    /// 
    /// # Arguments
    /// * `norm` - The normalized value from the scale.
    /// * `logical_max` - The maximum logical value or index (from scale.logical_max()).
    pub fn map_to_shape(&self, norm: f64, logical_max: f64) -> PointShape {
        match self {
            VisualMapper::Shape { custom_shapes } => {
                // Use custom list if provided, otherwise fallback to the built-in default shapes
                let shapes = custom_shapes
                    .as_deref()
                    .unwrap_or(PointShape::LEGEND_SHAPES);
                
                if shapes.is_empty() { 
                    return PointShape::Circle; 
                }

                // logical_max represents (number_of_categories - 1)
                // We re-scale the [0, 1] norm value back to the index space.
                let index = (norm * logical_max).round() as usize;
                
                // Use modulo to safely handle cases where there are more categories 
                // than available distinct shapes.
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