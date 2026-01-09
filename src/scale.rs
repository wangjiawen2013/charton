pub mod discrete;
pub mod linear;
pub mod log;
pub mod temporal;

use crate::error::ChartonError;
use self::linear::LinearScale;
use self::log::LogScale;
use self::discrete::DiscreteScale;
use self::temporal::TemporalScale;

// External time crate for date-time objects
use time::OffsetDateTime;

/// Represents an individual tick on an axis.
#[derive(Debug, Clone)]
pub struct Tick {
    /// The value in data space (e.g., 100.0, timestamp, or index).
    pub value: f64,
    /// The human-readable string representation (e.g., "100", "Jan 2026").
    pub label: String,
}

/// Represents the mathematical strategy used to map data values to visual positions.
#[derive(Clone, Debug, PartialEq)]
pub enum Scale {
    /// Linear mapping: equal data intervals result in equal visual distances.
    Linear,
    /// Logarithmic mapping: handles data spanning multiple orders of magnitude.
    Log,
    /// Categorical mapping: used for discrete strings or labels.
    Discrete,
    /// Temporal mapping: specifically for dates and times.
    Time,
}

/// Defines visual padding to be subtracted from the start and end of the range.
/// This ensures data points don't touch the canvas edges.
#[derive(Clone, Debug, Default)]
pub struct RangePadding {
    pub start: f64,
    pub end: f64,
}

impl RangePadding {
    pub fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }
}

/// A container for input data boundaries. 
/// It ensures type safety when initializing different scale types.
pub enum ScaleDomain {
    /// For Linear/Log: (min, max)
    Continuous(f64, f64),
    /// For Discrete: List of labels
    Categorical(Vec<String>),
    /// For Temporal: (start_time, end_time)
    Temporal(OffsetDateTime, OffsetDateTime),
}

/// The common interface for all scale types.
/// Defines the shared behavior for all coordinate mapping implementations.
pub trait ScaleTrait {
    /// Maps a raw value (or index/timestamp as f64) to a pixel coordinate.
    fn map(&self, value: f64) -> f64;
    /// Returns the domain boundaries in normalized f64.
    fn domain(&self) -> (f64, f64);
    /// Returns the output range boundaries. They are pixel coordinates in Charton.
    fn range(&self) -> (f64, f64);
    /// Generates a list of nice tick marks for the axis.
    /// * `count`: Suggested number of ticks.
    fn ticks(&self, count: usize) -> Vec<Tick>;
}

/// Factory function to instantiate a Scale with Range Padding applied.
pub fn create_scale(
    scale_type: &Scale,
    domain_data: ScaleDomain,
    range: (f64, f64),
    padding: RangePadding,
) -> Result<Box<dyn ScaleTrait>, ChartonError> {
    // Apply Range Padding
    // Adjusts the range so the actual mapping happens in an 'inner' box.
    // Now the range is a subset of the actual range.
    let adjusted_range = if range.0 < range.1 {
        (range.0 + padding.start, range.1 - padding.end)
    } else {
        // Handles inverted scales (like standard Y-axes in computer graphics)
        (range.0 - padding.start, range.1 + padding.end)
    };

    match scale_type {
        Scale::Linear => {
            if let ScaleDomain::Continuous(min, max) = domain_data {
                Ok(Box::new(LinearScale::new((min, max), adjusted_range)))
            } else {
                Err(ChartonError::Scale("Linear scale requires Continuous domain".into()))
            }
        },
        Scale::Log => {
            if let ScaleDomain::Continuous(min, max) = domain_data {
                Ok(Box::new(LogScale::new((min, max), adjusted_range, 10.0)?))
            } else {
                Err(ChartonError::Scale("Log scale requires Continuous domain".into()))
            }
        },
        Scale::Discrete => {
            if let ScaleDomain::Categorical(categories) = domain_data {
                Ok(Box::new(DiscreteScale::new(categories, adjusted_range)))
            } else {
                Err(ChartonError::Scale("Discrete scale requires Categorical domain".into()))
            }
        },
        Scale::Time => {
            if let ScaleDomain::Temporal(start, end) = domain_data {
                Ok(Box::new(TemporalScale::new((start, end), adjusted_range)))
            } else {
                Err(ChartonError::Scale("Time scale requires Temporal domain".into()))
            }
        }
    }
}
