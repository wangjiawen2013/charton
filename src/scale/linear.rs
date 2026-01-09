use super::{ScaleTrait, Tick};

/// A scale that performs linear interpolation between a continuous domain and a continuous range.
/// 
/// The `LinearScale` is the most common scale type, used for mapping quantitative data 
/// (like price, temperature, or speed) to a visual dimension (like pixels).
pub struct LinearScale {
    /// The input data boundaries: (min_value, max_value).
    domain: (f64, f64),
    /// The output visual boundaries: (start_pixel, end_pixel).
    range: (f64, f64),
}

impl LinearScale {
    /// Creates a new `LinearScale` with the specified domain and range.
    /// 
    /// # Arguments
    /// * `domain` - A tuple representing the minimum and maximum data values.
    /// * `range` - A tuple representing the starting and ending pixel coordinates.
    pub fn new(domain: (f64, f64), range: (f64, f64)) -> Self {
        Self { domain, range }
    }

    /// Calculates a "nice" step size for axis ticks based on the data range.
    /// 
    /// This algorithm finds a power-of-ten step (e.g., 0.1, 1, 10, 100) 
    /// and scales it by a "nice" factor (1, 2, or 5) to ensure that axis 
    /// labels are easy for humans to read.
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
    /// Maps a quantitative value from the domain to a pixel coordinate in the range.
    /// 
    /// Uses the linear formula: `y = r_min + (x - d_min) / (d_max - d_min) * (r_max - r_min)`
    fn map(&self, value: f64) -> f64 {
        let (d_min, d_max) = self.domain;
        let (r_min, r_max) = self.range;
        
        if (d_max - d_min).abs() < f64::EPSILON { 
            return r_min; 
        }
        
        let ratio = (value - d_min) / (d_max - d_min);
        r_min + ratio * (r_max - r_min)
    }

    /// Returns the data boundaries (min, max).
    fn domain(&self) -> (f64, f64) { self.domain }

    /// Returns the pixel boundaries (start, end).
    fn range(&self) -> (f64, f64) { self.range }

    /// Generates a list of human-friendly tick marks for an axis.
    /// 
    /// It automatically determines the appropriate decimal precision 
    /// based on the calculated step size to avoid messy labels like "1.0000000001".
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