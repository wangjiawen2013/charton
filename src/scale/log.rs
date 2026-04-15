use super::{ExplicitTick, Scale, ScaleDomain, ScaleTrait, Tick, mapper::VisualMapper};
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
    domain: (f64, f64),

    /// The logarithm base, typically 10.0 (common log) or 2.0 (binary log).
    base: f64,

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
    pub fn new(
        domain: (f64, f64),
        base: f64,
        mapper: Option<VisualMapper>,
    ) -> Result<Self, ChartonError> {
        if domain.0 <= 0.0 || domain.1 <= 0.0 {
            return Err(ChartonError::Scale(
                "Log scale domain must be strictly positive".into(),
            ));
        }
        if base <= 1.0 {
            return Err(ChartonError::Scale(
                "Log scale base must be greater than 1".into(),
            ));
        }
        Ok(Self {
            domain,
            base,
            mapper,
        })
    }

    /// Returns the logarithm base used by this scale.
    pub fn base(&self) -> f64 {
        self.base
    }
}

impl ScaleTrait for LogScale {
    fn scale_type(&self) -> Scale {
        Scale::Log
    }

    /// Transforms a value from the domain to a normalized [0, 1] ratio.
    ///
    /// The transformation follows the formula:
    /// $$normalized = \frac{\ln(val) - \ln(min)}{\ln(max) - \ln(min)}$$
    ///
    /// Values are clamped to the domain minimum to prevent undefined
    /// results from non-positive inputs.
    fn normalize(&self, value: f64) -> f64 {
        let (d_min, d_max) = self.domain;

        // Use natural log internally as it is more efficient and base-agnostic for ratios.
        let log_min = d_min.ln();
        let log_max = d_max.ln();

        // Clamp to avoid log(<= 0)
        let log_val = value.max(d_min).ln();

        let diff = log_max - log_min;
        if diff.abs() < f64::EPSILON {
            return 0.5;
        }

        (log_val - log_min) / diff
    }

    /// Continuous logarithmic scales return a default for categorical inputs.
    fn normalize_string(&self, _value: &str) -> f64 {
        f64::NAN
    }

    /// Returns the data boundaries (min, max).
    fn domain(&self) -> (f64, f64) {
        self.domain
    }

    /// For continuous log scales, the logical maximum is 1.0,
    /// representing 100% of the gradient or range.
    fn logical_max(&self) -> f64 {
        1.0
    }

    /// Returns the associated `VisualMapper` for this scale.
    fn mapper(&self) -> Option<&VisualMapper> {
        self.mapper.as_ref()
    }

    /// Generates logarithmic tick marks.
    ///
    /// This version focuses exclusively on "Major Ticks" (integer powers of the base).
    /// It includes a safety fallback to ensure at least two ticks are returned
    /// even if the data range is smaller than a single decade.
    fn suggest_ticks(&self, _count: usize) -> Vec<Tick> {
        let (min, max) = self.domain;
        let mut tick_values = Vec::new();

        // Logarithmic calculations to find the range of exponents
        let log_base = self.base.ln();
        let log_min = min.ln() / log_base;
        let log_max = max.ln() / log_base;

        let start_exp = log_min.floor() as i32;
        let end_exp = log_max.ceil() as i32;

        // 1. Generate Major Ticks (powers of the base)
        // We use a small epsilon (0.99/1.01) to account for floating point inaccuracies
        // ensuring that boundary values are captured.
        for exp in start_exp..=end_exp {
            let val = self.base.powi(exp);
            if val >= min * 0.99 && val <= max * 1.01 {
                tick_values.push(val);
            }
        }

        // 2. Fallback Logic
        // If the domain is very narrow (e.g., between 120 and 150), no integer power
        // of 10 exists within it. In such cases, we provide the min and max
        // as ticks so the axis isn't blank.
        if tick_values.len() < 2 {
            tick_values.clear();
            tick_values.push(min);
            tick_values.push(max);
        }

        // 3. Formatting
        // Delegate to the shared formatter which handles scientific notation
        // and converts raw f64 values into Tick objects.
        super::format_ticks(&tick_values)
    }

    /// Transforms user-defined explicit ticks into renderable Tick objects for Log scales.
    ///
    /// Note: Logarithmic scales only support positive values (v > 0).
    fn create_explicit_ticks(&self, explicit: &[ExplicitTick]) -> Vec<Tick> {
        let (min, max) = self.domain;

        // Logarithmic scales are sensitive near the boundaries.
        // We use a relative tolerance (e.g., 1%) rather than a fixed epsilon.
        let lower_bound = min * 0.9999999999;
        let upper_bound = max * 1.0000000001;

        let mut type_mismatch = 0;
        let mut out_of_domain = 0;

        let valid_values: Vec<f64> = explicit
            .iter()
            .filter_map(|tick| {
                match tick {
                    ExplicitTick::Continuous(val) => {
                        // 1. Check for mathematical validity (Log(v) requires v > 0)
                        // 2. Check if the value is within the scale's current domain
                        if *val > 0.0 && *val >= lower_bound && *val <= upper_bound {
                            Some(*val)
                        } else {
                            out_of_domain += 1;
                            None
                        }
                    }
                    // Mismatch if the user passed Discrete or Temporal to a Log scale
                    _ => {
                        type_mismatch += 1;
                        None
                    }
                }
            })
            .collect();

        // High-Performance Logging: Report issues after the loop
        if type_mismatch > 0 || out_of_domain > 0 {
            eprintln!(
                "Warning [LogScale]: Filtered {} ticks ({} type mismatch, {} out of domain or <= 0).",
                type_mismatch + out_of_domain,
                type_mismatch,
                out_of_domain
            );
        }

        // Reuse the shared formatting logic to ensure consistent labels
        // (especially important for scientific notation in log scales).
        super::format_ticks(&valid_values)
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

        if n == 0 {
            return Vec::new();
        }
        if n == 1 {
            return super::format_ticks(&[min]);
        }

        let log_min = min.ln();
        let log_max = max.ln();
        let log_step = (log_max - log_min) / (n - 1) as f64;

        let values: Vec<f64> = (0..n)
            .map(|i| {
                let log_val = if i == n - 1 {
                    log_max
                } else {
                    log_min + i as f64 * log_step
                };
                log_val.exp()
            })
            .collect();

        // Call super::format_ticks to ensure consistent axis-wide formatting (automatic scientific notation)
        super::format_ticks(&values)
    }
}
