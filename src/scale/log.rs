use super::{ScaleTrait, Tick};
use crate::error::ChartonError;

/// A scale that performs logarithmic transformation.
/// 
/// Logarithmic scales are ideal for visualizing data that spans several orders 
/// of magnitude. This implementation supports custom bases (defaulting to 10.0) 
/// and generates ticks at power-of-base intervals.
pub struct LogScale {
    /// The input data boundaries. Must be strictly positive.
    domain: (f64, f64),
    /// The output visual boundaries: (start_pixel, end_pixel).
    range: (f64, f64),
    /// The logarithm base, typically 10.0 or 2.0.
    base: f64,
}

impl LogScale {
    /// Creates a new `LogScale`.
    /// 
    /// # Errors
    /// Returns `ChartonError::Scale` if domain values are <= 0 or base <= 1.
    pub fn new(domain: (f64, f64), range: (f64, f64), base: f64) -> Result<Self, ChartonError> {
        if domain.0 <= 0.0 || domain.1 <= 0.0 {
            return Err(ChartonError::Scale("Log scale domain must be strictly positive".into()));
        }
        if base <= 1.0 {
            return Err(ChartonError::Scale("Log scale base must be greater than 1".into()));
        }
        Ok(Self { domain, range, base })
    }

    /// Returns the logarithm base.
    pub fn base(&self) -> f64 {
        self.base
    }
}

impl ScaleTrait for LogScale {
    /// Maps a value from the domain to a pixel coordinate using logarithmic interpolation.
    /// 
    /// The ratio is calculated in log-space:
    /// `ratio = (log(val) - log(min)) / (log(max) - log(min))`
    fn map(&self, value: f64) -> f64 {
        let (d_min, d_max) = self.domain;
        let (r_min, r_max) = self.range;

        // Use natural log for internal ratio calculation (base independent)
        let log_min = d_min.ln();
        let log_max = d_max.ln();
        // Clamp value to d_min to avoid log of non-positive numbers
        let log_val = value.max(d_min).ln();

        let ratio = (log_val - log_min) / (log_max - log_min);
        r_min + ratio * (r_max - r_min)
    }

    fn domain(&self) -> (f64, f64) { self.domain }
    fn range(&self) -> (f64, f64) { self.range }

    /// Generates logarithmic tick marks.
    /// 
    /// This method produces major ticks at powers of the base. If the number 
    /// of decades is small, it also injects minor ticks (like 2 and 5) to 
    /// fill the visual gaps.
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
            if val >= min * 0.99 && val <= max * 1.01 {
                tick_values.push(val);
            }
        }

        // 2. Generate Minor Ticks (if space permits)
        // If we have very few decades, add multipliers (2, 5)
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

        // 3. Format Labels
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
}