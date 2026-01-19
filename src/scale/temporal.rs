use super::{ScaleTrait, ScaleDomain, Tick};
use time::{OffsetDateTime, Duration};

/// A scale for temporal (date/time) data using the `time` crate.
/// 
/// It maps `OffsetDateTime` values to a normalized [0, 1] range by 
/// converting time points into Unix nanosecond timestamps and performing
/// linear interpolation.
/// 
/// Note: To support ggplot2-style padding, the domain stored here should 
/// represent the expanded time range (e.g., adding a few days or hours 
/// to the start/end of the data).
#[derive(Debug, Clone)]
pub struct TemporalScale {
    /// The input temporal boundaries (Start Time, End Time).
    /// These boundaries include any expansion padding.
    domain: (OffsetDateTime, OffsetDateTime),
}

impl TemporalScale {
    /// Creates a new `TemporalScale`.
    /// 
    /// # Arguments
    /// * `domain` - A tuple of (start_time, end_time). Should be expanded 
    ///              if padding is desired.
    pub fn new(domain: (OffsetDateTime, OffsetDateTime)) -> Self {
        Self { domain }
    }

    /// Helper method to transform an `OffsetDateTime` directly to a normalized [0, 1] value.
    pub fn normalize_time(&self, value: OffsetDateTime) -> f64 {
        self.normalize(value.unix_timestamp_nanos() as f64)
    }

    /// Determines the best time interval and a label format key based on the domain duration.
    /// 
    /// This heuristic ensures that axis labels remain readable regardless of whether 
    /// the data spans seconds or years.
    fn get_interval_info(&self) -> (Duration, &'static str) {
        let diff = self.domain.1 - self.domain.0;
        let seconds = diff.whole_seconds().abs();

        if seconds > 365 * 24 * 3600 {
            (Duration::days(365), "year")
        } else if seconds > 30 * 24 * 3600 {
            (Duration::days(30), "month")
        } else if seconds > 24 * 3600 {
            (Duration::days(1), "day")
        } else if seconds > 3600 {
            (Duration::hours(1), "hour")
        } else {
            (Duration::seconds(1), "second")
        }
    }
}

impl ScaleTrait for TemporalScale {
    /// Transforms a timestamp (as f64 nanoseconds) into a normalized [0, 1] ratio.
    /// 
    /// The transformation is linear based on the elapsed nanoseconds from the domain start.
    /// Since the domain is expanded, data points will naturally fall within a 
    /// sub-range of [0, 1], providing visual padding.
    fn normalize(&self, value: f64) -> f64 {
        let d_min = self.domain.0.unix_timestamp_nanos() as f64;
        let d_max = self.domain.1.unix_timestamp_nanos() as f64;

        let diff = d_max - d_min;
        if diff.abs() < f64::EPSILON {
            // Default to center if the time domain is a single point
            return 0.5;
        }

        (value - d_min) / diff
    }

    fn normalize_string(&self, _value: &str) -> f64 {
        0.0
    }

    /// Returns the domain boundaries converted to Unix nanosecond timestamps (f64).
    fn domain(&self) -> (f64, f64) {
        (
            self.domain.0.unix_timestamp_nanos() as f64,
            self.domain.1.unix_timestamp_nanos() as f64,
        )
    }

    /// Returns the maximum logical value for mapping.
    /// For temporal scales, this returns 1.0, treating the time range 
    /// as a continuous dimension for visual encodings like color gradients.
    fn logical_max(&self) -> f64 {
        1.0
    }

    /// Generates human-readable temporal ticks.
    /// 
    /// This method selects an appropriate time unit (e.g., Days, Months, Years)
    /// based on the domain duration and formats them using the `time` crate.
    /// Ticks are generated within the expanded domain.
    fn ticks(&self, _count: usize) -> Vec<Tick> {
        let (start, end) = self.domain;
        let (interval, format_key) = self.get_interval_info();
        
        let mut ticks = Vec::new();
        let mut curr = start;
        let mut iterations = 0;

        // Iteratively generate time steps until the end of the domain.
        // Capped at 50 iterations to maintain axis readability and performance.
        while curr <= end && iterations < 50 {
            let label = match format_key {
                "year" => curr.format(&time::macros::format_description!("[year]")),
                "month" => curr.format(&time::macros::format_description!("[year]-[month]")),
                "day" => curr.format(&time::macros::format_description!("[month]-[day]")),
                "hour" => curr.format(&time::macros::format_description!("[hour]:[minute]")),
                _ => curr.format(&time::macros::format_description!("[hour]:[minute]:[second]")),
            }.unwrap_or_else(|_| "??".to_string());

            ticks.push(Tick {
                value: curr.unix_timestamp_nanos() as f64,
                label,
            });
            
            // Advance time and guard against overflow
            match curr.checked_add(interval) {
                Some(next) => curr = next,
                None => break,
            }
            iterations += 1;
        }
        
        ticks
    }

    /// Returns the temporal domain as a ScaleDomain enum.
    /// 
    /// This allows the GuideManager to identify this scale as a time-based dimension
    /// and use the correct formatting and sampling logic for legends.
    fn get_domain_enum(&self) -> ScaleDomain {
        ScaleDomain::Temporal(self.domain.0, self.domain.1)
    }
}