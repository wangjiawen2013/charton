use super::{ScaleTrait, Scale, ScaleDomain, Tick, mapper::VisualMapper};

/// A scale that maps a continuous data domain to a normalized [0, 1] range.
/// 
/// The `LinearScale` is the workhorse of quantitative visualization. It performs
/// the mathematical transformation for positional channels (X, Y) and provides
/// the basis for continuous visual mappings (Color, Size).
///
/// In Charton's architecture, a `LinearScale` is often shared via `Arc` across
/// multiple layers to ensure they all use the same data-to-visual mapping.
#[derive(Debug, Clone)]
pub struct LinearScale {
    /// The input data boundaries: (min_value, max_value).
    /// These values typically include expansion padding calculated during training.
    domain: (f64, f64),

    /// The optional visual mapper that defines how the normalized [0, 1] value
    /// is converted into concrete aesthetics like colors or physical sizes.
    mapper: Option<VisualMapper>,
}

impl LinearScale {
    /// Creates a new `LinearScale` with a specified domain and an optional visual mapper.
    /// 
    /// # Arguments
    /// * `domain` - A tuple of (min, max) representing the expanded data range.
    /// * `mapper` - An optional `VisualMapper` for non-positional aesthetics.
    pub fn new(domain: (f64, f64), mapper: Option<VisualMapper>) -> Self {
        Self { domain, mapper }
    }

    /// Calculates a "nice" step size for axis ticks (e.g., 0.1, 0.2, 0.5, 1.0).
    /// 
    /// This ensures that the intervals between ticks are intuitive for human readers.
    /// It uses the range of the domain and a target tick count to find the 
    /// optimal power-of-ten interval.
    fn calculate_nice_step(&self, count: usize) -> f64 {
        let (min, max) = self.domain;
        let range = max - min;
        
        // Safety check for single-point domains or identical boundaries.
        if range.abs() < 1e-12 {
            return 1.0; 
        }

        let rough_step = range / (count.max(2) as f64);
        
        // Magnitude (power of 10) of the rough step.
        let exp = 10f64.powf(rough_step.log10().floor());
        
        // Normalize the step to the [1, 10] range to pick the best "nice" factor.
        let f = rough_step / exp;

        let nice = if f < 1.5 { 1.0 }
            else if f < 3.0 { 2.0 }
            else if f < 7.0 { 5.0 }
            else { 10.0 };

        nice * exp
    }
}

impl ScaleTrait for LinearScale {
    fn scale_type(&self) -> Scale { Scale::Linear }

    /// Maps a raw data value to a normalized [0.0, 1.0] float.
    /// 
    /// Formula: `(value - min) / (max - min)`. 
    /// If the value is within the expansion padding, the result will be 
    /// slightly inside the [0, 1] range.
    fn normalize(&self, value: f64) -> f64 {
        let (d_min, d_max) = self.domain;
        let diff = d_max - d_min;
        
        if diff.abs() < f64::EPSILON { 
            return 0.5; // Default to center for zero-width domains.
        }
        
        (value - d_min) / diff
    }

    /// Continuous linear scales return a fallback for categorical string inputs.
    fn normalize_string(&self, _value: &str) -> f64 {
        0.0
    }

    /// Returns the data boundaries used by this scale.
    fn domain(&self) -> (f64, f64) { 
        self.domain 
    }

    /// For continuous scales, the logical maximum is always 1.0, 
    /// representing 100% of the mapping range.
    fn logical_max(&self) -> f64 {
        1.0
    }

    /// Returns a reference to the internal `VisualMapper`.
    /// 
    /// This is used by marks (e.g., a bubble) to determine their specific 
    /// visual property (e.g., color) after the data value has been normalized.
    fn mapper(&self) -> Option<&VisualMapper> {
        self.mapper.as_ref()
    }

    /// Generates human-readable tick marks based on the domain.
    /// 
    /// This method automatically adjusts the precision of the string labels 
    /// based on the magnitude of the calculated nice step.
    fn ticks(&self, count: usize) -> Vec<Tick> {
        let (min, max) = self.domain;
        let step = self.calculate_nice_step(count);
        let tolerance = step * 1e-9;
        
        let start = (min / step).ceil() * step;
        let mut values = Vec::new();
        let mut curr = start;

        let mut iterations = 0;
        while curr <= max + tolerance && iterations < count * 2 {
            let clean_val = if curr.abs() < 1e-12 { 0.0 } else { curr };
            values.push(clean_val);
            
            curr += step;
            iterations += 1;
        }
        
        // Ensure consistent axis-wide formatting (automatic scientific notation)
        super::format_ticks(&values)
    }

    /// Returns the domain specification for chart guide and legend logic.
    fn get_domain_enum(&self) -> ScaleDomain {
        ScaleDomain::Continuous(self.domain.0, self.domain.1)
    }

    /// Equidistant sampling used for legends that require fixed density.
    /// 
    /// Unlike `ticks`, this guarantees exactly `n` points, even if the 
    /// values are not "pretty" decimals.
    fn sample_n(&self, n: usize) -> Vec<Tick> {
        let (min, max) = self.domain;
        
        if n == 0 { return Vec::new(); }
        if n == 1 {
            return super::format_ticks(&[min]);
        }

        let step = (max - min) / (n - 1) as f64;
        let values: Vec<f64> = (0..n)
            .map(|i| {
                let val = if i == n - 1 { max } else { min + i as f64 * step };
                if val.abs() < 1e-12 { 0.0 } else { val }
            })
            .collect();

        super::format_ticks(&values)
    }
}