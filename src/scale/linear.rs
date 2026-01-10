use super::{ScaleTrait, Tick};

/// A scale that maps a continuous data domain to a normalized [0, 1] range.
/// 
/// In the Charton layered system, the `LinearScale` handles the mathematical 
/// transformation of quantitative data. It does not know about pixels; 
/// that mapping is deferred to the Coordinate system.
/// 
/// Note: The domain stored here should be the expanded domain (including padding)
/// to ensure that data points are mapped correctly within the visual area.
pub struct LinearScale {
    /// The input data boundaries: (min_value, max_value).
    /// Following ggplot2 principles, these values usually include a small 
    /// expansion buffer beyond the raw data range.
    domain: (f64, f64),
}

impl LinearScale {
    /// Creates a new `LinearScale` with the specified domain.
    /// 
    /// # Arguments
    /// * `domain` - A tuple representing the minimum and maximum data values.
    ///              This should be the expanded range if padding is desired.
    pub fn new(domain: (f64, f64)) -> Self {
        Self { domain }
    }

    /// Calculates a "nice" step size for axis ticks based on the data domain.
    /// 
    /// This algorithm finds a power-of-ten step (e.g., 0.1, 1, 10, 100) 
    /// and scales it by a "nice" factor (1, 2, or 5) to ensure that axis 
    /// labels are intuitive for human readers.
    fn calculate_nice_step(&self, count: usize) -> f64 {
        let (min, max) = self.domain;
        let range = max - min;
        
        // Handle edge case where domain is a single point or invalid
        if range.abs() < 1e-12 {
            return 1.0; 
        }

        // Calculate a rough step size based on the requested tick count
        let rough_step = range / (count.max(2) as f64);
        
        // Find the magnitude (power of 10) of the step
        let exp = 10f64.powf(rough_step.log10().floor());
        
        // Determine the fractional part to pick the closest "nice" number
        let f = rough_step / exp;

        let nice = if f < 1.5 { 1.0 }
            else if f < 3.0 { 2.0 }
            else if f < 7.0 { 5.0 }
            else { 10.0 };

        nice * exp
    }
}

impl ScaleTrait for LinearScale {
    /// Transforms a quantitative value from the domain to a normalized [0, 1] value.
    /// 
    /// Implementation of the linear formula: `normalized = (x - d_min) / (d_max - d_min)`
    /// Because the domain is expanded, raw data points will be mapped to a range 
    /// slightly smaller than [0, 1] (e.g., [0.05, 0.95]), creating visual padding.
    fn normalize(&self, value: f64) -> f64 {
        let (d_min, d_max) = self.domain;
        
        let diff = d_max - d_min;
        if diff.abs() < f64::EPSILON { 
            // If the domain is a single point, we map it to the center (0.5)
            return 0.5; 
        }
        
        (value - d_min) / diff
    }

    fn normalize_string(&self, value: &str) -> f64 {
        0.0
    }

    /// Returns the data boundaries (min, max).
    fn domain(&self) -> (f64, f64) { 
        self.domain 
    }

    /// Returns the maximum logical value for mapping.
    /// For continuous scales like Linear, this is always 1.0 to represent 
    /// a full 0% to 100% gradient range.
    fn domain_max(&self) -> f64 {
        1.0
    }

    /// Generates a list of human-friendly tick marks for an axis.
    /// 
    /// This method identifies appropriate decimal precision based on 
    /// the step size to provide clean, readable labels.
    fn ticks(&self, count: usize) -> Vec<Tick> {
        let (min, max) = self.domain;
        let step = self.calculate_nice_step(count);
        
        // Determine precision: The number of decimal places is derived 
        // from the magnitude of the step size.
        let precision = (-(step.log10().floor()) as i32).max(0) as usize;
        
        // Tolerance used to include the end boundary despite floating point errors.
        let tolerance = step * 1e-9;
        
        // Find the first nice tick value equal to or greater than the domain minimum.
        let start = (min / step).ceil() * step;
        let mut ticks = Vec::new();
        let mut curr = start;

        // Iteratively generate ticks until the domain maximum is reached.
        // Capped at 100 iterations as a safety guard against infinite loops.
        let mut iterations = 0;
        while curr <= max + tolerance && iterations < 100 {
            // Clean up values extremely close to zero (e.g., 1.23e-17 -> 0.0)
            let clean_val = if curr.abs() < 1e-12 { 0.0 } else { curr };
            
            ticks.push(Tick {
                value: clean_val,
                label: format!("{:.1$}", clean_val, precision),
            });
            
            curr += step;
            iterations += 1;
        }
        
        ticks
    }
}