pub mod discrete;
pub mod linear;
pub mod log;
pub mod temporal;
pub mod mapper;

use crate::error::ChartonError;
use self::linear::LinearScale;
use self::log::LogScale;
use self::discrete::DiscreteScale;
use self::temporal::TemporalScale;
use self::mapper::VisualMapper;

use std::sync::Arc;
use time::OffsetDateTime;
use polars::datatypes::AnyValue;
use polars::prelude::*;
use dyn_clone::{DynClone, clone_trait_object};

/// Defines how much a scale's domain should be expanded beyond the data limits.
/// 
/// Following ggplot2's expansion system, it consists of a multiplicative factor 
/// and an additive constant. This prevents data marks from clipping at the 
/// edges of the coordinate system.
#[derive(Debug, Clone, Copy)]
pub struct Expansion {
    /// Multiplicative factors (lower_mult, upper_mult). 
    /// e.g., (0.05, 0.05) adds 5% padding relative to the data range.
    pub mult: (f32, f32),
    /// Additive constants in data units (lower_add, upper_add).
    pub add: (f32, f32),
}

impl Default for Expansion {
    /// Default expansion is 5% on each side, which is standard for continuous scales.
    fn default() -> Self {
        Self { 
            mult: (0.05, 0.05), 
            add: (0.0, 0.0) 
        }
    }
}

/// Represents an individual tick mark on an axis or legend.
#[derive(Debug, Clone)]
pub struct Tick {
    /// The raw value in data space.
    pub value: f32,
    /// The formatted label for display (e.g., "1k", "2026-01").
    pub label: String,
}

/// The mathematical strategy for mapping data to a [0, 1] normalized space.
#[derive(Clone, Debug, PartialEq)]
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
    ) -> Result<Float32Chunked, ChartonError> {
        match self {
            Scale::Discrete => {
                let out: Float32Chunked = series.iter().map(|val| {
                    let norm = match val {
                        AnyValue::String(s) => scale_trait.normalize_string(s),
                        AnyValue::StringOwned(s) => scale_trait.normalize_string(&s),
                        _ => scale_trait.normalize_string(&val.to_string()),
                    };
                    Some(norm)
                }).collect();
                Ok(out)
            }
            _ => {
                let casted = series.cast(&DataType::Float32)
                    .map_err(|e| ChartonError::Data(e.to_string()))?;
                let ca = casted.f32().unwrap();
                Ok(ca.apply(|opt_v| opt_v.map(|v| scale_trait.normalize(v))))
            }
        }
    }
}

/// A type-safe container for data boundaries.
#[derive(Debug, Clone)]
pub enum ScaleDomain {
    Continuous(f32, f32),
    Discrete(Vec<String>),
    Temporal(OffsetDateTime, OffsetDateTime),
}

/// The primary interface for all scale implementations.
/// 
/// A `ScaleTrait` is responsible for two things:
/// 1. Mathematical: Mapping raw data values to a normalized [0.0, 1.0] range.
/// 2. Visual: Linking to a `VisualMapper` that converts normalized values to colors/shapes.
pub trait ScaleTrait: DynClone + std::fmt::Debug + Send + Sync {
    /// Maps a numerical value to a normalized [0, 1] value.
    fn normalize(&self, value: f32) -> f32;

    /// Maps a discrete string to a normalized [0, 1] value.
    fn normalize_string(&self, value: &str) -> f32;

    /// Returns the data-space boundaries of the scale.
    fn domain(&self) -> (f32, f32);

    /// Returns the maximum logical index for discrete scales, or 1.0 for continuous.
    fn logical_max(&self) -> f32;

    /// Returns the visual mapper associated with this scale, if any.
    /// This allows marks to resolve colors, shapes, or sizes directly from the scale.
    fn mapper(&self) -> Option<&VisualMapper>;

    /// Generates suggested tick marks for axes or legends.
    fn ticks(&self, count: usize) -> Vec<Tick>;

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
    expand: Expansion,
    mapper: Option<VisualMapper>, // Added: Associate visual mapping logic at creation
) -> Result<Arc<dyn ScaleTrait>, ChartonError> {
    let scale: Box<dyn ScaleTrait> = match scale_type {
        Scale::Linear => {
            if let ScaleDomain::Continuous(min, max) = domain_data {
                let range = max - min;
                let lower_padding = range * expand.mult.0 + expand.add.0;
                let upper_padding = range * expand.mult.1 + expand.add.1;
                Box::new(LinearScale::new((min - lower_padding, max + upper_padding), mapper))
            } else {
                return Err(ChartonError::Scale("Linear scale requires Continuous domain".into()));
            }
        },
        Scale::Log => {
            if let ScaleDomain::Continuous(min, max) = domain_data {
                let log_min = min.ln();
                let log_max = max.ln();
                let log_range = log_max - log_min;
                let expanded_min = (log_min - log_range * expand.mult.0).exp();
                let expanded_max = (log_max + log_range * expand.mult.1).exp();
                Box::new(LogScale::new((expanded_min, expanded_max), 10.0, mapper)?)
            } else {
                return Err(ChartonError::Scale("Log scale requires Continuous domain".into()));
            }
        },
        Scale::Discrete => {
            if let ScaleDomain::Discrete(categories) = domain_data {
                Box::new(DiscreteScale::new(categories, expand, mapper))
            } else {
                return Err(ChartonError::Scale("Discrete scale requires Categorical domain".into()));
            }
        },
        Scale::Temporal => {
            if let ScaleDomain::Temporal(start, end) = domain_data {
                let diff_secs = (end - start).as_seconds_f32();
                let lower_pad = time::Duration::seconds_f32(diff_secs * expand.mult.0 + expand.add.0);
                let upper_pad = time::Duration::seconds_f32(diff_secs * expand.mult.1 + expand.add.1);
                Box::new(TemporalScale::new((start - lower_pad, end + upper_pad), mapper))
            } else {
                return Err(ChartonError::Scale("Time scale requires Temporal domain".into()));
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
) -> f32 {
    match scale_type {
        Scale::Discrete => {
            match value {
                AnyValue::String(s) => scale_trait.normalize_string(s),
                AnyValue::StringOwned(s) => scale_trait.normalize_string(s.as_str()),
                _ => scale_trait.normalize_string(&value.to_string()),
            }
        }
        _ => {
            value.try_extract::<f32>()
                .map(|v| scale_trait.normalize(v))
                .unwrap_or(0.0)
        }
    }
}

clone_trait_object!(ScaleTrait);