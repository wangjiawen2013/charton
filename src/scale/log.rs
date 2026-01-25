use super::{ScaleTrait, ScaleDomain, Tick, mapper::VisualMapper};
use crate::error::ChartonError;

/// A scale that performs logarithmic transformation.
/// 
/// Logarithmic scales are essential for visualizing data that spans several orders 
/// of magnitude. This implementation transforms strictly positive data into a 
/// normalized [0, 1] space based on log-ratios.
/// 
/// In Charton's architecture, the `LogScale` can be associated with a `VisualMapper`
/// to allow properties like bubble size or color intensity to scale logarithmically,
/// which often provides a more truthful representation of exponential growth.
#[derive(Debug, Clone)]
pub struct LogScale {
    /// The input data boundaries. Must be strictly positive (> 0).
    /// These represent the visual limits of the axis in raw data units.
    domain: (f32, f32),
    
    /// The logarithm base, typically 10.0 (common log) or 2.0 (binary log).
    base: f32,

    /// The optional visual mapper used to convert normalized log-ratios 
    /// into aesthetics like colors or sizes.
    mapper: Option<VisualMapper>,
}

impl LogScale {
    /// Creates a new `LogScale`.
    /// 
    /// # Arguments
    /// * `domain` - Strictly positive (min, max) data range.
    /// * `base` - The logarithm base (must be > 1.0).
    /// * `mapper` - Optional visual logic for aesthetic mapping.
    ///
    /// # Errors
    /// Returns `ChartonError::Scale` if domain values are <= 0 or base <= 1.
    pub fn new(domain: (f32, f32), base: f32, mapper: Option<VisualMapper>) -> Result<Self, ChartonError> {
        if domain.0 <= 0.0 || domain.1 <= 0.0 {
            return Err(ChartonError::Scale("Log scale domain must be strictly positive".into()));
        }
        if base <= 1.0 {
            return Err(ChartonError::Scale("Log scale base must be greater than 1".into()));
        }
        Ok(Self { domain, base, mapper })
    }

    /// Returns the logarithm base used by this scale.
    pub fn base(&self) -> f32 {
        self.base
    }
}

impl ScaleTrait for LogScale {
    /// Transforms a value from the domain to a normalized [0, 1] ratio.
    /// 
    /// The transformation follows the formula:
    /// $$normalized = \frac{\ln(val) - \ln(min)}{\ln(max) - \ln(min)}$$
    /// 
    /// Values are clamped to the domain minimum to prevent undefined 
    /// results from non-positive inputs.
    fn normalize(&self, value: f32) -> f32 {
        let (d_min, d_max) = self.domain;

        // Use natural log internally as it is more efficient and base-agnostic for ratios.
        let log_min = d_min.ln();
        let log_max = d_max.ln();
        
        // Clamp to avoid log(<= 0)
        let log_val = value.max(d_min).ln();

        let diff = log_max - log_min;
        if diff.abs() < f32::EPSILON {
            return 0.5;
        }

        (log_val - log_min) / diff
    }

    /// Continuous logarithmic scales return a default for categorical inputs.
    fn normalize_string(&self, _value: &str) -> f32 {
        0.0
    }

    /// Returns the data boundaries (min, max).
    fn domain(&self) -> (f32, f32) { 
        self.domain 
    }

    /// For continuous log scales, the logical maximum is 1.0, 
    /// representing 100% of the gradient or range.
    fn logical_max(&self) -> f32 {
        1.0
    }

    /// Returns the associated `VisualMapper` for this scale.
    fn mapper(&self) -> Option<&VisualMapper> {
        self.mapper.as_ref()
    }

    /// Generates logarithmic tick marks.
    /// 
    /// This algorithm prioritizes major ticks at powers of the base (e.g., 1, 10, 100).
    /// If the data spans few decades, it injects minor ticks (multipliers like 2 and 5)
    /// to provide better visual context.
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

        // 2. Generate Minor Ticks (if density is low)
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

        // 3. Format Labels with logic for scientific vs decimal notation
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

    /// Returns the domain boundaries wrapped in the continuous enum variant.
    fn get_domain_enum(&self) -> ScaleDomain {
        let (min, max) = self.domain;
        ScaleDomain::Continuous(min, max)
    }

    /// Force-samples the domain into N points equidistant in log-space.
    /// 
    /// This is crucial for creating accurate legends for log scales. If we 
    /// sampled linearly, the legend would not reflect the geometric 
    /// progression of the data.
    fn sample_n(&self, n: usize) -> Vec<Tick> {
        let (min, max) = self.domain;
        
        if n == 0 { return Vec::new(); }
        if n == 1 {
            return vec![Tick { value: min, label: format!("{:.1}", min) }];
        }

        let log_min = min.ln();
        let log_max = max.ln();
        let log_step = (log_max - log_min) / (n - 1) as f32;

        (0..n).map(|i| {
            let log_val = if i == n - 1 { log_max } else { log_min + i as f32 * log_step };
            let val = log_val.exp();

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