use crate::core::layer::{MarkRenderer, RenderBackend};
use crate::core::context::SharedRenderingContext;
use crate::visual::shape::PointShape;
use crate::error::ChartonError;
use polars::prelude::*;

/// `MarkPoint` is responsible for driving the rendering loop of a scatter plot.
/// It translates data rows into graphical primitives via the `RenderBackend`.
pub struct MarkPoint {
    /// The column name in the DataFrame used for the X-axis position.
    pub x_col: String,
    /// The column name in the DataFrame used for the Y-axis position.
    pub y_col: String,
    /// Optional: The column name used to map data values to colors.
    pub color_col: Option<String>,
    /// Optional: The column name used to map data values to shapes.
    pub shape_col: Option<String>,
    /// Optional: The column name used to map data values to sizes.
    pub size_col: Option<String>,
    
    // --- Constant Fallbacks ---
    /// Default radius or half-width of the point if no size mapping is provided.
    pub default_size: f64,
    /// Default hex color (e.g., "#4682B4") if no color mapping is provided.
    pub default_color: String,
    /// Default geometric shape if no shape mapping is provided.
    pub default_shape: PointShape,
    /// Global opacity for all points in this layer (0.0 to 1.0).
    pub opacity: f64,
}

impl MarkRenderer for MarkPoint {
    /// The main entry point for rendering the layer.
    /// It iterates through the data, calculates visual attributes, and calls the backend.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // 1. Access the underlying data from the context.
        // The data is assumed to be processed/filtered by the core engine.
        let data = context.get_data(); 
        
        // 2. Extract the primary coordinate series.
        // We use unwrap() here assuming the Training Phase validated column existence.
        let x_series = data.column(&self.x_col).map_err(|e| ChartonError::Data(e.to_string()))?.f64().unwrap();
        let y_series = data.column(&self.y_col).map_err(|e| ChartonError::Data(e.to_string()))?.f64().unwrap();

        // 3. Iterate through every row in the DataFrame.
        for i in 0..data.height() {
            let x_raw = x_series.get(i);
            let y_raw = y_series.get(i);

            if let (Some(xv), Some(yv)) = (x_raw, y_raw) {
                
                // --- COORDINATE TRANSFORMATION ---
                // First, normalize the raw data [min, max] -> [0.0, 1.0] using the Scale.
                // Then, use the Coordinate System to map [0.0, 1.0] to Pixel Space (px, py).
                // This automatically handles 'Swapped Axes' or 'Polar Coordinates'.
                let x_norm = context.coord.x_scale().normalize(xv);
                let y_norm = context.coord.y_scale().normalize(yv);
                let (px, py) = context.transform(x_norm, y_norm);

                // --- VISUAL MAPPING (Aesthetics) ---
                // Retrieve the color, size, and shape for this specific row.
                // The `context.aesthetics` object holds the pre-computed Mappers.
                let color = context.aesthetics.map_color(data, i).unwrap_or(self.default_color.clone());
                let size = context.aesthetics.map_size(data, i).unwrap_or(self.default_size);
                let shape = context.aesthetics.map_shape(data, i).unwrap_or(self.default_shape.clone());

                // --- DRAWING DISPATCH ---
                // Based on the resolved shape, we call the appropriate Backend primitive.
                self.emit_draw_call(backend, &shape, px, py, size, &color, self.opacity);
            }
        }
        Ok(())
    }
}

impl MarkPoint {
    /// A helper function to translate geometric shapes into backend drawing commands.
    /// This keeps the `render_marks` loop clean and focused on data logic.
    fn emit_draw_call(
        &self,
        backend: &mut dyn RenderBackend,
        shape: &PointShape,
        x: f64,
        y: f64,
        size: f64, // 'size' usually refers to the radius or half-width
        color: &str,
        opacity: f64,
    ) {
        match shape {
            PointShape::Circle => {
                backend.draw_circle(x, y, size, color, opacity);
            }
            PointShape::Square => {
                let side = size * 2.0;
                backend.draw_rect(x - size, y - size, side, side, color);
            }
            PointShape::Triangle => {
                // Calculation for an equilateral triangle pointing upwards
                let h = size * 1.732; // Height
                let pts = vec![
                    (x, y - h * (2.0/3.0)),       // Top vertex
                    (x - size, y + h * (1.0/3.0)), // Bottom left
                    (x + size, y + h * (1.0/3.0)), // Bottom right
                ];
                backend.draw_polygon(&pts, color, opacity);
            }
            PointShape::Diamond => {
                let pts = vec![
                    (x, y - size), // Top
                    (x + size, y), // Right
                    (x, y + size), // Bottom
                    (x - size, y), // Left
                ];
                backend.draw_polygon(&pts, color, opacity);
            }
            PointShape::Star => {
                // Generates a 5-pointed star by alternating inner and outer radii
                let mut pts = Vec::with_capacity(10);
                let outer_r = size;
                let inner_r = size * 0.4;
                for j in 0..10 {
                    let r = if j % 2 == 0 { outer_r } else { inner_r };
                    let angle = std::f64::consts::PI / 2.0 + (j as f64) * std::f64::consts::PI / 5.0;
                    pts.push((x + r * angle.cos(), y - r * angle.sin()));
                }
                backend.draw_polygon(&pts, color, opacity);
            }
            // Add more shapes (Hexagon, Pentagon, etc.) here as needed...
            _ => {
                // Fallback to circle for unsupported shapes
                backend.draw_circle(x, y, size, color, opacity);
            }
        }
    }
}