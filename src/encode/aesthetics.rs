use crate::scale::ScaleTrait;
use crate::scale::mapper::VisualMapper;
use crate::coordinate::CoordinateTrait;
use crate::error::ChartonError;
use crate::visual::shape::PointShape;
use polars::prelude::*;

/// `AestheticData` encapsulates the final visual attributes for a chart.
/// All coordinates are normalized relative to the coordinate system's logic.
pub(crate) struct AestheticData {
    pub(crate) x_normalized: Vec<f64>,
    pub(crate) y_normalized: Vec<f64>,
    pub(crate) shapes: Option<Vec<PointShape>>,
    pub(crate) colors: Option<Vec<String>>,
    pub(crate) sizes: Option<Vec<f64>>,
}

impl AestheticData {
    /// Constructs a new `AestheticData` by applying scales and mappers.
    /// This is now decoupled from specific coordinate systems.
    pub(crate) fn new<T: crate::mark::Mark>(
        chart: &crate::chart::Chart<T>,
        coord_system: &dyn CoordinateTrait, // 👈 Accept any coordinate system
        color_mapper: Option<(&VisualMapper, &dyn ScaleTrait)>,
        shape_mapper: Option<(&VisualMapper, &dyn ScaleTrait)>,
        size_mapper: Option<(&VisualMapper, &dyn ScaleTrait)>,
    ) -> Result<Self, ChartonError> {
        
        // --- 1. Position Encodings (X, Y) ---
        // These are the core geometric foundations of any chart
        let x_field = &chart.encoding.x.as_ref().unwrap().field;
        let y_field = &chart.encoding.y.as_ref().unwrap().field;

        let x_series = chart.data.column(x_field)?;
        let y_series = chart.data.column(y_field)?;

        // Note: coord_system provides access to its internal scales via the trait
        let x_normalized = Self::normalize_series(&x_series, coord_system.get_x_scale())?;
        let y_normalized = Self::normalize_series(&y_series, coord_system.get_y_scale())?;

        // --- 2. Color Aesthetic ---
        let colors = if let (Some(color_enc), Some((mapper, scale))) = (&chart.encoding.color, color_mapper) {
            let color_series = chart.data.column(&color_enc.field)?;
            let norm_vals = Self::normalize_series(&color_series, scale)?;
            let d_max = scale.domain_max();
            
            Some(norm_vals.into_iter().map(|v| mapper.map_to_color(v, d_max)).collect())
        } else {
            None
        };

        // --- 3. Shape Aesthetic ---
        let shapes = if let (Some(shape_enc), Some((mapper, scale))) = (&chart.encoding.shape, shape_mapper) {
            let shape_series = chart.data.column(&shape_enc.field)?;
            let norm_vals = Self::normalize_series(&shape_series, scale)?;
            let d_max = scale.domain_max();
            
            Some(norm_vals.into_iter().map(|v| mapper.map_to_shape(v, d_max)).collect())
        } else {
            None
        };

        // --- 4. Size Aesthetic ---
        let sizes = if let (Some(size_enc), Some((mapper, scale))) = (&chart.encoding.size, size_mapper) {
            let size_series = chart.data.column(&size_enc.field)?;
            let norm_vals = Self::normalize_series(&size_series, scale)?;
            
            Some(norm_vals.into_iter().map(|v| mapper.map_to_size(v)).collect())
        } else if let Some(size_enc) = &chart.encoding.size {
            // Fallback: Simple linear map if no mapper is provided
            let size_series = chart.data.column(&size_enc.field)?;
            let vals = size_series.cast(&DataType::Float64)?.f64()?.into_no_null_iter().collect::<Vec<_>>();
            let min = vals.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max = vals.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let range = max - min;
            
            Some(vals.into_iter().map(|v| {
                let norm = if range.abs() > 1e-9 { (v - min) / range } else { 0.5 };
                2.0 + norm * 10.0 
            }).collect())
        } else {
            None
        };

        Ok(Self {
            x_normalized,
            y_normalized,
            shapes,
            colors,
            sizes,
        })
    }

    /// Internal helper to transform a Polars Series into normalized [0, 1] values
    fn normalize_series(
        series: &Series, 
        scale: &dyn ScaleTrait
    ) -> Result<Vec<f64>, ChartonError> {
        match series.dtype() {
            DataType::String => {
                let ca = series.str()?;
                Ok(ca.into_no_null_iter()
                    .map(|s| scale.normalize_string(s))
                    .collect())
            },
            _ => {
                let f64_series = series.cast(&DataType::Float64)?;
                let ca = f64_series.f64()?;
                Ok(ca.into_no_null_iter()
                    .map(|v| scale.normalize(v))
                    .collect())
            }
        }
    }
}