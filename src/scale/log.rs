use super::{ScaleTrait, ScaleDomain, Tick};
use crate::error::ChartonError;

/// A scale that performs logarithmic transformation.
/// 
/// Logarithmic scales are ideal for visualizing data that spans several orders 
/// of magnitude. This implementation transforms data into a normalized [0, 1] 
/// space based on log-ratios.
/// 
/// Note: To support visual padding, the domain stored here should be 
/// expanded in log-space. This prevents data points from sticking to 
/// the axis boundaries.
#[derive(Debug, Clone)]
pub struct LogScale {
    /// The input data boundaries. Must be strictly positive.
    /// In an expanded scale, these represent the visual limits of the axis.
    domain: (f32, f32),
    /// The logarithm base, typically 10.0 or 2.0.
    base: f32,
}

impl LogScale {
    /// Creates a new `LogScale`.
    /// 
    /// # Errors
    /// Returns `ChartonError::Scale` if domain values are <= 0 or base <= 1.
    pub fn new(domain: (f32, f32), base: f32) -> Result<Self, ChartonError> {
        if domain.0 <= 0.0 || domain.1 <= 0.0 {
            return Err(ChartonError::Scale("Log scale domain must be strictly positive".into()));
        }
        if base <= 1.0 {
            return Err(ChartonError::Scale("Log scale base must be greater than 1".into()));
        }
        Ok(Self { domain, base })
    }

    /// Returns the logarithm base.
    pub fn base(&self) -> f32 {
        self.base
    }
}

impl ScaleTrait for LogScale {
    /// Transforms a value from the domain to a normalized [0, 1] ratio using log interpolation.
    /// 
    /// The ratio is calculated as:
    /// $$ratio = \frac{\log_{base}(val) - \log_{base}(min)}{\log_{base}(max) - \log_{base}(min)}$$
    /// Because the domain is expanded, raw data values will map to a 
    /// sub-range within [0, 1], providing visual breathing room.
    fn normalize(&self, value: f32) -> f32 {
        let (d_min, d_max) = self.domain;

        // Use natural log for internal ratio calculation (it is base-agnostic)
        let log_min = d_min.ln();
        let log_max = d_max.ln();
        
        // Clamp value to d_min to avoid log of non-positive numbers
        let log_val = value.max(d_min).ln();

        let diff = log_max - log_min;
        if diff.abs() < f32::EPSILON {
            return 0.5;
        }

        (log_val - log_min) / diff
    }

    fn normalize_string(&self, _value: &str) -> f32 {
        0.0
    }

    /// Returns the data boundaries (min, max).
    fn domain(&self) -> (f32, f32) { 
        self.domain 
    }

    /// Returns the maximum logical value for mapping.
    /// For continuous log scales, this returns 1.0 to support continuous 
    /// visual mapping (like color gradients) over the log-transformed range.
    fn logical_max(&self) -> f32 {
        1.0
    }

    /// Generates logarithmic tick marks.
    /// 
    /// Produces major ticks at powers of the base. If the number of decades 
    /// is small relative to the requested count, it injects minor ticks 
    /// (multipliers of 2 and 5) to fill visual gaps.
    fn ticks(&self, count: usize) -> Vec<Tick> {
        let (min, max) = self.domain;
        let mut tick_values = Vec::new();
        
        let log_base = self.base.ln();
        let log_min = min.ln() / log_base;
        let log_max = max.ln() / log_base;

        let start_exp = log_min.floor() as i32;
        let end_exp = log_max.ceil() as i32;

        // 1. Generate Major Ticks (powers of the base)
        for exp in start_exp..=end_exp {
            let val = self.base.powi(exp);
            // Allow a small buffer for float precision
            if val >= min * 0.99 && val <= max * 1.01 {
                tick_values.push(val);
            }
        }

        // 2. Generate Minor Ticks (if space permits)
        let n_decades = (end_exp - start_exp).abs();
        if n_decades < (count as i32) {
            let mut minor_ticks = Vec::new();
            let multipliers = [2.0, 5.0];
            
            for exp in (start_exp - 1)..=end_exp {
                let base_val = self.base.powi(exp);
                for &m in &multipliers {
                    let val = base_val * m;
                    if val > min && val < max {
                        minor_ticks.push(val);
                    }
                }
            }
            tick_values.extend(minor_ticks);
            tick_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            tick_values.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        }

        // 3. Format Labels with appropriate scientific or decimal notation
        tick_values.into_iter().map(|v| {
            let label = if v >= 1e6 || v <= 1e-3 {
                format!("{:.1e}", v)
            } else if v >= 1.0 {
                format!("{:.0}", v)
            } else {
                format!("{:.3}", v).trim_end_matches('0').trim_end_matches('.').to_string()
            };
            Tick { value: v, label }
        }).collect()
    }

    /// Implementation for LogScale.
    /// Returns the logarithmic domain boundaries as a ScaleDomain::Continuous.
    /// 
    /// Note: This returns the raw data limits (expanded in log-space), 
    /// allowing the Guide system to treat it as a continuous numeric range.
    fn get_domain_enum(&self) -> ScaleDomain {
        let (min, max) = self.domain;
        ScaleDomain::Continuous(min, max)
    }

    /// Force-samples the scale domain into N points, equidistant in log-space.
    /// 
    /// This is essential for log-scale legends (like Size). If we sampled linearly, 
    /// the visual representation would be heavily biased. By sampling in log-space, 
    /// each step represents a constant geometric multiplier (e.g., each circle is 
    /// 2x the value of the previous one).
    fn sample_n(&self, n: usize) -> Vec<Tick> {
        let (min, max) = self.domain;
        
        if n == 0 { return Vec::new(); }
        if n == 1 {
            return vec![Tick { value: min, label: format!("{:.1}", min) }];
        }

        // 1. Transform boundaries to log-space
        let log_min = min.ln();
        let log_max = max.ln();
        let log_step = (log_max - log_min) / (n - 1) as f32;

        (0..n).map(|i| {
            // 2. Interpolate in log-space and exponentiate back to raw space
            let log_val = if i == n - 1 { log_max } else { log_min + i as f32 * log_step };
            let val = log_val.exp();

            // 3. Apply the same formatting logic as the standard `ticks` method
            let label = if val >= 1e6 || val <= 1e-3 {
                format!("{:.1e}", val)
            } else if val >= 1.0 {
                format!("{:.1}", val).trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                format!("{:.3}", val).trim_end_matches('0').trim_end_matches('.').to_string()
            };

            Tick { value: val, label }
        }).collect()
    }
}