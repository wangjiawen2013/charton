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

    // Private helper for consistent formatting
    fn format_dt(&self, dt: OffsetDateTime, format_key: &str) -> String {
        match format_key {
            "year" => dt.format(&time::macros::format_description!("[year]")),
            "month" => dt.format(&time::macros::format_description!("[year]-[month]")),
            "day" => dt.format(&time::macros::format_description!("[month]-[day]")),
            "hour" => dt.format(&time::macros::format_description!("[hour]:[minute]")),
            _ => dt.format(&time::macros::format_description!("[hour]:[minute]:[second]")),
        }.unwrap_or_else(|_| "??".to_string())
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
            let label = self.format_dt(curr, format_key);

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

    /// Force-samples the temporal domain into N equidistant time points.
    /// 
    /// This ensures that time-based legends (like Size or Color) display a 
    /// balanced set of samples across the entire duration, regardless of 
    /// standard calendar intervals (like months or years).
    fn sample_n(&self, n: usize) -> Vec<Tick> {
        let (start_dt, end_dt) = self.domain;
        
        if n == 0 { return Vec::new(); }
        if n == 1 {
            let (_, format_key) = self.get_interval_info();
            return vec![Tick {
                value: start_dt.unix_timestamp_nanos() as f64,
                label: self.format_dt(start_dt, format_key),
            }];
        }

        let start_ns = start_dt.unix_timestamp_nanos() as f64;
        let end_ns = end_dt.unix_timestamp_nanos() as f64;
        let step_ns = (end_ns - start_ns) / (n - 1) as f64;

        let (_, format_key) = self.get_interval_info();

        (0..n).map(|i| {
            let current_ns = if i == n - 1 { end_ns } else { start_ns + i as f64 * step_ns };
            
            // Convert nanoseconds back to OffsetDateTime
            let dt = OffsetDateTime::from_unix_timestamp_nanos(current_ns as i128)
                .unwrap_or(start_dt);

            Tick {
                value: current_ns,
                label: self.format_dt(dt, format_key),
            }
        }).collect()
    }
}