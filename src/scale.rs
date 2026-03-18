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
use crate::error::ChartonError;

use polars::datatypes::AnyValue;
use polars::prelude::*;
use std::sync::{Arc, RwLock};
use time::OffsetDateTime;

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

/// The "Input": What the user explicitly requests.
#[derive(Debug, Clone, PartialEq)]
pub enum ExplicitTick {
    Continuous(f64),
    Discrete(String),
    Temporal(OffsetDateTime),
}

/// A trait to allow various collection types to be converted
/// into a vector of TickValues automatically.
pub trait IntoExplicitTicks {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick>;
}

impl IntoExplicitTicks for Vec<f64> {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter().map(ExplicitTick::Continuous).collect()
    }
}

impl<const N: usize> IntoExplicitTicks for [f64; N] {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter().map(ExplicitTick::Continuous).collect()
    }
}

impl<const N: usize> IntoExplicitTicks for [&str; N] {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter()
            .map(|s| ExplicitTick::Discrete(s.to_string()))
            .collect()
    }
}

impl IntoExplicitTicks for Vec<&str> {
    fn into_explicit_ticks(self) -> Vec<ExplicitTick> {
        self.into_iter()
            .map(|s| ExplicitTick::Discrete(s.to_string()))
            .collect()
    }
}

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
    /// High-performance vectorized normalization using Polars.
    ///
    /// This bypasses row-by-row iteration for numerical data, applying the
    /// scale transformation across entire memory chunks at once.
    pub fn normalize_series(
        &self,
        scale_trait: &dyn ScaleTrait,
        series: &Series,
    ) -> Result<Float64Chunked, ChartonError> {
        match self {
            Scale::Discrete => {
                let out: Float64Chunked = series
                    .iter()
                    .map(|val| {
                        let norm = match val {
                            AnyValue::String(s) => scale_trait.normalize_string(s),
                            AnyValue::StringOwned(s) => scale_trait.normalize_string(&s),
                            _ => scale_trait.normalize_string(&val.to_string()),
                        };
                        Some(norm)
                    })
                    .collect();
                Ok(out)
            }
            _ => {
                let casted = series
                    .cast(&DataType::Float64)
                    .map_err(|e| ChartonError::Data(e.to_string()))?;
                let ca = casted.f64().unwrap();
                Ok(ca.apply(|opt_v| opt_v.map(|v| scale_trait.normalize(v))))
            }
        }
    }
}

/// A type-safe container for data boundaries.
#[derive(Debug, Clone, PartialEq)]
pub enum ScaleDomain {
    Continuous(f64, f64),
    Discrete(Vec<String>),
    Temporal(OffsetDateTime, OffsetDateTime),
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
            if let ScaleDomain::Temporal(start, end) = domain_data {
                let diff_secs = (end - start).as_seconds_f64();
                let lower_pad =
                    time::Duration::seconds_f64(diff_secs * expansion.mult.0 + expansion.add.0);
                let upper_pad =
                    time::Duration::seconds_f64(diff_secs * expansion.mult.1 + expansion.add.1);
                Box::new(TemporalScale::new(
                    (start - lower_pad, end + upper_pad),
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

/// Utility for extracting a normalized value from an AnyValue.
pub fn get_normalized_value(
    scale_trait: &dyn ScaleTrait,
    scale_type: &Scale,
    value: &AnyValue,
) -> f64 {
    match scale_type {
        Scale::Discrete => match value {
            AnyValue::String(s) => scale_trait.normalize_string(s),
            AnyValue::StringOwned(s) => scale_trait.normalize_string(s.as_str()),
            _ => scale_trait.normalize_string(&value.to_string()),
        },
        _ => value
            .try_extract::<f64>()
            .map(|v| scale_trait.normalize(v))
            .unwrap_or(0.0),
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
