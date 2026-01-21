use crate::visual::shape::PointShape;
use crate::visual::color::{ColorMap, ColorPalette, SingleColor};
use crate::scale::Scale;
use crate::theme::Theme;

/// Defines how normalized data values [0.0, 1.0] are mapped to physical visual properties.
/// 
/// This enum acts as the final stage of the scale pipeline, converting abstract 
/// mathematical ratios into concrete types like `SingleColor`, `PointShape`, or `f32`.
#[derive(Debug, Clone)]
pub enum VisualMapper {
    /// Continuous color mapping for numerical data (Gradients).
    ContinuousColor {
        map: ColorMap,
    },
    /// Discrete color mapping for categorical data (Palettes).
    DiscreteColor {
        palette: ColorPalette,
    },
    /// Geometric shape mapping for categorical data.
    Shape {
        /// Optional list of shapes. If None, defaults to `PointShape::LEGEND_SHAPES`.
        custom_shapes: Option<Vec<PointShape>>,
    },
    /// Size mapping for numerical data (Linear Interpolation).
    Size {
        /// Tuple representing (min_size, max_size) in physical units (pixels/points).
        range: (f32, f32),
    },
}

impl VisualMapper {
    /// Creates a default color mapper based on whether the scale is discrete or continuous.
    /// 
    /// Inherits the aesthetic preferences (Palette/Map) from the provided `Theme`.
    pub fn new_color_default(scale_type: &Scale, theme: &Theme) -> Self {
        match scale_type {
            Scale::Discrete => {
                VisualMapper::DiscreteColor {
                    palette: theme.palette,
                }
            }
            _ => {
                VisualMapper::ContinuousColor {
                    map: theme.color_map,
                }
            }
        }
    }

    /// Creates a default size mapper with a specified physical range.
    pub fn new_size_default(min: f32, max: f32) -> Self {
        VisualMapper::Size {
            range: (min, max),
        }
    }

    /// Creates a default shape mapper using the standard geometric shapes.
    pub fn new_shape_default() -> Self {
        VisualMapper::Shape {
            custom_shapes: None,
        }
    }

    /// Maps a normalized value [0.0, 1.0] to a `SingleColor`.
    /// 
    /// This returns a `SingleColor` object which contains both the CSS string for SVG
    /// and pre-calculated RGBA values for GPU backends like wgpu.
    /// 
    /// # Arguments
    /// * `norm` - The normalized value from the scale (0.0 to 1.0).
    /// * `logical_max` - For discrete scales, represents the highest index (n-1).
    pub fn map_to_color(&self, norm: f32, logical_max: f32) -> SingleColor {
        match self {
            VisualMapper::ContinuousColor { map } => {
                // Interpolates within the continuous gradient
                map.get_color(norm)
            },
            VisualMapper::DiscreteColor { palette } => {
                // Maps the 0-1 norm back to a specific palette index
                let index = (norm * logical_max).round() as usize;
                palette.get_color(index)
            }
            // Fallback: Returns Opaque Black if color mapping is called on a non-color mapper
            _ => SingleColor::default(), 
        }
    }

    /// Maps a normalized value to a `PointShape`.
    /// 
    /// # Arguments
    /// * `norm` - The normalized value from the scale.
    /// * `logical_max` - The maximum index (number_of_categories - 1).
    pub fn map_to_shape(&self, norm: f32, logical_max: f32) -> PointShape {
        match self {
            VisualMapper::Shape { custom_shapes } => {
                let shapes = custom_shapes
                    .as_deref()
                    .unwrap_or(PointShape::LEGEND_SHAPES);
                
                if shapes.is_empty() { 
                    return PointShape::Circle; 
                }

                let index = (norm * logical_max).round() as usize;
                
                // Use modulo to cycle through shapes if categories exceed available shapes.
                // PointShape is Copy-safe and provides .gpu_id() for wgpu logic.
                shapes[index % shapes.len()]
            }
            _ => PointShape::Circle,
        }
    }

    /// Maps a normalized value to a physical size (radius or width).
    /// 
    /// Performs linear interpolation: size = min + norm * (max - min).
    pub fn map_to_size(&self, norm: f32) -> f32 {
        match self {
            VisualMapper::Size { range } => {
                range.0 + norm * (range.1 - range.0)
            }
            // Default size if no size mapping is specified
            _ => 5.0, 
        }
    }
}