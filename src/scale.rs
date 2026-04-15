pub mod discrete;
pub mod linear;
pub mod log;
pub mod mapper;
pub mod temporal;

use self::discrete::DiscreteScale;
use self::linear::LinearScale;
use self::log::LogScale;
use self::mapper::VisualMapper;
use self::temporal::TemporalScale;
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use std::sync::{Arc, RwLock};
use time::OffsetDateTime;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Defines how much a scale's domain should be expanded beyond the data limits.
///
/// Following ggplot2's expansion system, it consists of a multiplicative factor
/// and an additive constant. This prevents data marks from clipping at the
/// edges of the coordinate system.
#[derive(Debug, Clone, Copy)]
pub struct Expansion {
    /// Multiplicative factors (lower_mult, upper_mult).
    /// e.g., (0.05, 0.05) adds 5% padding relative to the data range.
    pub mult: (f64, f64),
    /// Additive constants in data units (lower_add, upper_add).
    pub add: (f64, f64),
}

impl Default for Expansion {
    /// Default expansion is 5% on each side, which is standard for continuous scales.
    fn default() -> Self {
        Self {
            mult: (0.05, 0.05),
            add: (0.0, 0.0),
        }
    }
}

/// Represents an individual tick mark on an axis or legend.
#[derive(Debug, Clone)]
pub struct Tick {
    /// The raw value in data space.
    pub value: f64,
    /// The formatted label for display (e.g., "1k", "2026-01").
    pub label: String,
}

/// Represents a user-defined tick value for various scale types.
///
/// This enum acts as a container for data points that the user wants to
/// explicitly highlight on an axis, regardless of the scale's internal logic.
#[derive(Debug, Clone, PartialEq)]
pub enum ExplicitTick {
    /// For linear or logarithmic scales (e.g., price, temperature, generic f64).
    Continuous(f64),

    /// For categorical or ordinal scales (e.g., "Product A", "Category B").
    Discrete(String),

    /// The "Universal Bridge" for high-precision temporal data.
    /// Accepts raw Unix nanoseconds from external sources like Chrono or Polars.
    /// This avoids floating-point precision loss for nanosecond-level timestamps.
    Timestamp(i64),

    /// Native support for high-level date-time objects from the `time` crate.
    /// Provides timezone awareness and calendar-accurate formatting.
    Temporal(OffsetDateTime),
}

/// A trait that allows various collection types (Vec, Arrays, etc.) to be
/// automatically converted into a vector of `ExplicitTick` variants.
///
/// This trait enables a polymorphic API where users can pass raw primitive
/// types directly into axis-configuration methods.
pub trait IntoExplicitTicks {
    /// Consumes the collection and returns a vector of standardized `ExplicitTick`s.
    fn into_explicit_ticks(self) -> Vec<ExplicitTick>;
}

/// Implementation for standard floating-point numbers.
/// Maps to `ExplicitTick::Continuous`.
impl IntoExplicitTicks for Vec<f64> {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter().map(ExplicitTick::Continuous).collect()
    }
}

/// Array support for floating-point numbers to allow fixed-size inputs.
impl<const N: usize> IntoExplicitTicks for [f64; N] {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter().map(ExplicitTick::Continuous).collect()
    }
}

/// Implementation for string slices, commonly used for categorical labels.
/// Maps to `ExplicitTick::Discrete`.
impl IntoExplicitTicks for Vec<&str> {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter()
            .map(|s| ExplicitTick::Discrete(s.to_string()))
            .collect()
    }
}

/// Implementation for raw integers.
/// In the context of Charton, these are treated as high-precision nanosecond timestamps.
/// Maps to `ExplicitTick::Timestamp`.
impl IntoExplicitTicks for Vec<i64> {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter().map(ExplicitTick::Timestamp).collect()
    }
}

/// Array support for raw integers (timestamps).
impl<const N: usize> IntoExplicitTicks for [i64; N] {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter().map(ExplicitTick::Timestamp).collect()
    }
}

/// Implementation for native `time::OffsetDateTime` objects.
/// Maps to `ExplicitTick::Temporal`.
impl IntoExplicitTicks for Vec<OffsetDateTime> {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter().map(ExplicitTick::Temporal).collect()
    }
}

/// The mathematical strategy for mapping data to a [0, 1] normalized space.
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Scale {
    Linear,
    Log,
    Discrete,
    Temporal,
}

impl Scale {
    /// High-performance normalization of an entire ColumnVector into a vector of f64.
    ///
    /// This method maps raw data values to the normalized [0, 1] coordinate space.
    /// It leverages `rayon` for parallel processing to ensure low latency even with
    /// massive datasets (millions of rows).
    /// High-performance normalization of an entire ColumnVector into a vector of f64.
    pub fn normalize_column(
        &self,
        scale_trait: &dyn ScaleTrait,
        column: &crate::core::data::ColumnVector,
    ) -> Vec<Option<f64>> {
        (0..column.len())
            .maybe_into_par_iter()
            .map(|i| {
                match self {
                    // Discrete scale: Force everything to string and normalize.
                    Scale::Discrete => column.get_str(i).map(|s| scale_trait.normalize_string(&s)),

                    // Continuous scales (Linear/Log): Use the numerical interface.
                    Scale::Linear | Scale::Log => {
                        column.get_f64(i).map(|v| scale_trait.normalize(v))
                    }

                    // Temporal scale: Also uses get_f64 (which returns nanoseconds).
                    Scale::Temporal => column.get_f64(i).map(|v| scale_trait.normalize(v)),
                }
            })
            .collect()
    }
}

/// A type-safe container for data boundaries.
#[derive(Debug, Clone, PartialEq)]
pub enum ScaleDomain {
    Continuous(f64, f64),
    Discrete(Vec<String>),
    Temporal(i64, i64), // (raw start nanoseconds, raw end nanoseconds)
}

/// The primary interface for all scale implementations.
///
/// A `ScaleTrait` is responsible for two things:
/// 1. Mathematical: Mapping raw data values to a normalized [0.0, 1.0] range.
/// 2. Visual: Linking to a `VisualMapper` that converts normalized values to colors/shapes.
pub trait ScaleTrait: std::fmt::Debug + Send + Sync {
    /// Returns the high-level category of this scale.
    /// Used for branching logic in legends and axis rendering.
    fn scale_type(&self) -> Scale;

    /// Maps a numerical value to a normalized [0, 1] value.
    /// Value must be f64 to keep the time accurate.
    fn normalize(&self, value: f64) -> f64;

    /// Maps a discrete string to a normalized [0, 1] value.
    fn normalize_string(&self, value: &str) -> f64;

    /// Returns the data-space boundaries of the scale.
    fn domain(&self) -> (f64, f64);

    /// Returns the maximum logical index for discrete scales, or 1.0 for continuous.
    fn logical_max(&self) -> f64;

    /// Returns the visual mapper associated with this scale, if any.
    /// This allows marks to resolve colors, shapes, or sizes directly from the scale.
    fn mapper(&self) -> Option<&VisualMapper>;

    /// Generates suggested tick marks for axes or legends.
    fn suggest_ticks(&self, count: usize) -> Vec<Tick>;

    /// Generates user-requested ticks.
    fn create_explicit_ticks(&self, explicit: &[ExplicitTick]) -> Vec<Tick>;

    /// Returns the domain specification as an enum for guide generation.
    fn get_domain_enum(&self) -> ScaleDomain;

    /// Equidistant sampling of the domain.
    fn sample_n(&self, n: usize) -> Vec<Tick>;
}

/// Factory function to create a fully initialized scale.
///
/// It resolves the domain expansion and encapsulates the concrete implementation
/// inside an `Arc` for efficient sharing between chart layers.
pub fn create_scale(
    scale_type: &Scale,
    domain_data: ScaleDomain,
    expansion: Expansion,
    mapper: Option<VisualMapper>, // Added: Associate visual mapping logic at creation
) -> Result<Arc<dyn ScaleTrait>, ChartonError> {
    let scale: Box<dyn ScaleTrait> = match scale_type {
        Scale::Linear => {
            if let ScaleDomain::Continuous(min, max) = domain_data {
                let range = max - min;
                let lower_padding = range * expansion.mult.0 + expansion.add.0;
                let upper_padding = range * expansion.mult.1 + expansion.add.1;
                Box::new(LinearScale::new(
                    (min - lower_padding, max + upper_padding),
                    mapper,
                ))
            } else {
                return Err(ChartonError::Scale(
                    "Linear scale requires Continuous domain".into(),
                ));
            }
        }
        Scale::Log => {
            if let ScaleDomain::Continuous(min, max) = domain_data {
                let log_min = min.ln();
                let log_max = max.ln();
                let log_range = log_max - log_min;
                let expanded_min = (log_min - log_range * expansion.mult.0).exp();
                let expanded_max = (log_max + log_range * expansion.mult.1).exp();
                Box::new(LogScale::new((expanded_min, expanded_max), 10.0, mapper)?)
            } else {
                return Err(ChartonError::Scale(
                    "Log scale requires Continuous domain".into(),
                ));
            }
        }
        Scale::Discrete => {
            if let ScaleDomain::Discrete(categories) = domain_data {
                Box::new(DiscreteScale::new(categories, expansion, mapper))
            } else {
                return Err(ChartonError::Scale(
                    "Discrete scale requires Categorical domain".into(),
                ));
            }
        }
        Scale::Temporal => {
            if let ScaleDomain::Temporal(min_ns, max_ns) = domain_data {
                // 1. Convert to nanoseconds immediately to perform high-precision expansion
                let diff_ns = (max_ns - min_ns) as f64;

                // 2. Calculate padding in nanoseconds (f64 for multiplication, then to i64)
                // Note: expansion.add.0 is assumed to be in seconds, so we multiply by 1e9
                let lower_pad_ns = (diff_ns * expansion.mult.0 + expansion.add.0 * 1e9) as i64;
                let upper_pad_ns = (diff_ns * expansion.mult.1 + expansion.add.1 * 1e9) as i64;

                // 3. Pass the raw i64 nanoseconds to the constructor
                Box::new(TemporalScale::new(
                    (min_ns - lower_pad_ns, max_ns + upper_pad_ns),
                    mapper,
                ))
            } else {
                return Err(ChartonError::Scale(
                    "Time scale requires Temporal domain".into(),
                ));
            }
        }
    };

    Ok(Arc::from(scale))
}

/// Utility for extracting a normalized [0, 1] value from an `ExplicitTick`.
///
/// This function acts as a bridge between raw data variants and the mathematical
/// scale logic, enforcing the "Interpretation Mode" dictated by the scale type:
///
/// - **Discrete Scales**: Operates in "Universal Discrete" mode. Every input variant
///   is coerced into its string representation to be mapped against categorical labels.
/// - **Continuous Scales (Linear, Log, Temporal)**: Treats inputs as numerical values.
///   Temporal types are converted to nanosecond-precision floats.
///
/// Returns `f64::NAN` if the conversion is impossible or the value cannot be mapped,
/// ensuring invalid data is safely ignored by the renderer rather than defaulting to the origin.
pub fn get_normalized_value(
    scale_trait: &dyn ScaleTrait,
    scale_type: &Scale,
    value: &ExplicitTick,
) -> f64 {
    match scale_type {
        // --- 1. DISCRETE SCALE ---
        // Mirroring the 'Universal Discrete' logic: everything is a string.
        Scale::Discrete => {
            let label = match value {
                ExplicitTick::Discrete(s) => s.clone(),
                ExplicitTick::Continuous(v) => v.to_string(),
                ExplicitTick::Timestamp(ts) => ts.to_string(),
                ExplicitTick::Temporal(dt) => dt.to_string(),
            };
            scale_trait.normalize_string(&label)
        }

        // --- 2. CONTINUOUS SCALES (Linear, Log, Temporal) ---
        // These all rely on f64 mapping (Temporal uses nanoseconds as f64).
        _ => {
            match value {
                ExplicitTick::Continuous(v) => scale_trait.normalize(*v),
                ExplicitTick::Timestamp(ns) => scale_trait.normalize(*ns as f64),
                ExplicitTick::Temporal(dt) => {
                    scale_trait.normalize(dt.unix_timestamp_nanos() as f64)
                }
                ExplicitTick::Discrete(_) => unreachable!("Discrete values are blocked for cotinuous scales by validataion"),
            }
        }
    }
}

/// A universal tick formatter following data visualization best practices.
/// Suitable for linear, power, and log scales.
pub(crate) fn format_ticks(values: &[f64]) -> Vec<Tick> {
    if values.is_empty() {
        return vec![];
    }

    // 1. Detect if scientific notation is required for the entire set.
    // Standard practice: Use 'E' for values >= 10,000 or <= 0.001.
    let use_sci = values.iter().any(|&v| {
        let a = v.abs();
        a != 0.0 && (a >= 10000.0 || a <= 0.001)
    });

    // 2. Calculate the step size to derive precision.
    // The "Step" dictates how many decimals are needed for distinctness.
    let step = if values.len() > 1 {
        (values[1] - values[0]).abs()
    } else {
        values[0].abs()
    };

    // 3. Precision calculation based on notation mode.
    let mut precision = if use_sci {
        let max_val = values.iter().map(|v| v.abs()).fold(0.0, f64::max);
        let magnitude = if max_val > 0.0 {
            max_val.log10().floor()
        } else {
            0.0
        };
        let step_mag = if step > 0.0 {
            step.log10().floor()
        } else {
            magnitude
        };
        ((magnitude - step_mag).max(0.0) as usize).clamp(0, 6)
    } else if step > 0.0 && step < 0.9999 {
        ((-step.log10()).ceil() as usize).clamp(0, 6)
    } else {
        0
    };

    // 4. Initial Formatting Pass
    let mut labels: Vec<String> = values
        .iter()
        .map(|&v| {
            if use_sci {
                format!("{:.*e}", precision, v).replace("e", "E")
            } else {
                format!("{:.*}", precision, v)
            }
        })
        .collect();

    // 5. Global Redundancy Check (The "Smart" part)
    // If every single label in the set has '.000' decimals, they are all removed.
    // This preserves alignment if even ONE label needs the decimal.
    if precision > 0 {
        let all_redundant = labels.iter().all(|l| {
            if let Some(dot_idx) = l.find('.') {
                // Check characters between '.' and end (or 'E' suffix)
                let end_idx = l.find('E').unwrap_or(l.len());
                l[dot_idx + 1..end_idx].chars().all(|c| c == '0')
            } else {
                true
            }
        });

        if all_redundant {
            precision = 0;
            labels = values
                .iter()
                .map(|&v| {
                    if use_sci {
                        format!("{:.*e}", precision, v).replace("e", "E")
                    } else {
                        format!("{:.*}", precision, v)
                    }
                })
                .collect();
        }
    }

    values
        .iter()
        .zip(labels)
        .map(|(&v, l)| Tick { value: v, label: l })
        .collect()
}

/// A thread-safe wrapper for the resolved scale that handles the cloning logic.
///
/// Since std::sync::RwLock does not implement Clone, we manually implement it
/// by creating a new lock that shares the same internal Arc pointer.
#[derive(Debug)]
pub struct ResolvedScale(pub(crate) RwLock<Option<Arc<dyn ScaleTrait>>>);

impl ResolvedScale {
    pub fn new(scale: Option<Arc<dyn ScaleTrait>>) -> Self {
        Self(RwLock::new(scale))
    }

    /// A helper to create an empty scale without messy type casting in callers
    pub fn none() -> Self {
        Self::new(None)
    }
}

impl Clone for ResolvedScale {
    fn clone(&self) -> Self {
        // Step 1: Acquire a read lock on the current scale.
        let guard = self.0.read().unwrap();

        // Step 2: Clone the Option<Arc<...>>.
        // This only increments the reference count of the Arc, which is very fast.
        let inner_clone = guard.clone();

        // Step 3: Wrap the cloned reference in a brand new RwLock.
        Self(RwLock::new(inner_clone))
    }
}
