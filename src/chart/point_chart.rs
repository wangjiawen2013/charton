use crate::chart::common::{Chart, SharedRenderingContext};
use crate::chart::data_processor::ProcessedChartData;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::mark::point::MarkPoint;
use crate::render::color_legend_renderer;
use crate::render::colorbar_renderer;
use crate::render::point_renderer;
use crate::render::shape_legend_renderer;
use crate::render::size_legend_renderer;
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;
use polars::prelude::*;

// Implementation specific to MarkPoint with additional methods
impl Chart<MarkPoint> {
    /// Create a new point mark chart
    ///
    /// Initializes the chart with a `MarkPoint` instance, enabling scatter plot rendering.
    /// This method must be called to configure the chart for displaying data as individual points.
    pub fn mark_point(mut self) -> Self {
        self.mark = Some(MarkPoint::new());
        self
    }

    /// Set the fill color for points
    ///
    /// Configures the interior color of the data points. When `None` is provided,
    /// the system will use default coloring or palette-based colors if color encoding
    /// is applied.
    ///
    /// # Arguments
    /// * `color` - Optional `SingleColor` specifying the fill color for points
    pub fn with_point_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the shape for points
    ///
    /// Defines the geometric shape used to represent data points. Various shapes
    /// can be used to differentiate categories or add visual interest to the plot.
    ///
    /// # Arguments
    /// * `shape` - A `PointShape` enum value specifying the point geometry
    pub fn with_point_shape(mut self, shape: PointShape) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.shape = shape;
        self.mark = Some(mark);
        self
    }

    /// Set the size for points
    ///
    /// Controls the dimensions of the data points. Larger values create more prominent
    /// points that are easier to see but may overlap in dense plots.
    ///
    /// # Arguments
    /// * `size` - A `f64` value representing the point size in pixels
    pub fn with_point_size(mut self, size: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.size = size;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for points
    ///
    /// Adjusts the transparency of the data points. Values range from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Useful for overlapping points or emphasizing certain data.
    ///
    /// # Arguments
    /// * `opacity` - A `f64` value between 0.0 and 1.0 representing the point opacity
    pub fn with_point_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke color for points
    ///
    /// Defines the outline color of the data points. When `None` is provided,
    /// no stroke will be rendered around the points.
    ///
    /// # Arguments
    /// * `stroke` - Optional `SingleColor` specifying the stroke color for point outlines
    pub fn with_point_stroke(mut self, stroke: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke = stroke;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width for points
    ///
    /// Specifies the thickness of the point outlines in pixels. Larger values create
    /// thicker borders around the data points.
    ///
    /// # Arguments
    /// * `stroke_width` - A `f64` value representing the stroke width in pixels
    pub fn with_point_stroke_width(mut self, stroke_width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke_width = stroke_width;
        self.mark = Some(mark);
        self
    }

    // Render point elements
    fn render_points(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Process chart data
        let processed_data = ProcessedChartData::new(self, context.coord_system)?;

        // Extract the mark from Option
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering rules".to_string())
        })?;

        // Render points for each data item
        for i in 0..processed_data.x_vals.len() {
            // Get the correct data values
            let x_val = processed_data.x_transformed_vals[i];
            let y_val = processed_data.y_transformed_vals[i];

            // Apply the mappers to get pixel coordinates
            let x_mapped = (context.x_mapper)(x_val);
            let y_mapped = (context.y_mapper)(y_val);

            // When axes are swapped, we need to swap the coordinates passed to render_point
            let (x, y) = if context.swapped_axes {
                (y_mapped, x_mapped) // Swap x and y when axes are swapped
            } else {
                (x_mapped, y_mapped) // Normal order when axes are not swapped
            };

            // Determine color for this text based on the scale type
            let fill_color = if let Some((scale_type, color_values)) = &processed_data.color_info {
                let &value = &color_values[i];
                match scale_type {
                    // Continuous color mapping using colormap
                    crate::coord::Scale::Linear | crate::coord::Scale::Log => {
                        // Use the existing colormap implementation
                        let color = self.mark_cmap.get_color(value);
                        Some(SingleColor::new(&color))
                    }
                    // Discrete color mapping using palette
                    crate::coord::Scale::Discrete => {
                        // Use the existing palette implementation
                        let color = self.mark_palette.get_color(value as usize);
                        Some(SingleColor::new(&color))
                    }
                }
            } else {
                // No color encoding channel, use default color
                mark.color.clone()
            };

            let shape = if let Some(ref shape_vals) = processed_data.shape_vals {
                let shape_str = &shape_vals[i]; // Direct indexing

                // Get unique values to maintain consistent mapping
                let unique_shapes_series =
                    Series::new("shape_vals".into(), shape_vals.clone()).unique_stable()?;

                let unique_shapes: Vec<String> = unique_shapes_series
                    .str()?
                    .into_no_null_iter()
                    .map(|s| s.to_string())
                    .collect();

                // Find index of this category in the unique values
                let shape_index = unique_shapes
                    .iter()
                    .position(|x| x == shape_str)
                    .ok_or_else(|| {
                        ChartonError::Internal("shape not found in unique values".to_string())
                    })?;

                // Available shapes for mapping (same order as in shape legend)
                let available_shapes = crate::visual::shape::PointShape::LEGEND_SHAPES;

                // Systematically map category index to shape
                let shape_type_index = shape_index % available_shapes.len();
                available_shapes[shape_type_index].clone()
            } else {
                mark.shape()
            };

            let size = if let Some(ref normalized_sizes) = processed_data.normalized_sizes {
                *normalized_sizes.get(i).ok_or_else(|| {
                    ChartonError::Internal("Index out of bounds in normalized_sizes".to_string())
                })?
            } else {
                mark.size
            };

            // Render the point
            point_renderer::render_point(
                svg,
                x,
                y,
                &fill_color,
                &shape,
                size,
                mark.opacity,
                &mark.stroke,
                mark.stroke_width,
            )?;
        }

        Ok(())
    }
}

// Implementation of MarkRenderer for Chart<MarkPoint>
impl crate::chart::common::MarkRenderer for Chart<MarkPoint> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_points(svg, context)
    }
}

impl crate::chart::common::LegendRenderer for Chart<MarkPoint> {
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Render colorbar if needed
        colorbar_renderer::render_colorbar(svg, self, theme, context)?;

        // Render color legend if needed
        color_legend_renderer::render_color_legend(svg, self, theme, context)?;

        // Render size legend if needed
        size_legend_renderer::render_size_legend(svg, self, theme, context)?;

        // Render shape legend if needed
        shape_legend_renderer::render_shape_legend(svg, self, theme, context)?;

        Ok(())
    }
}
