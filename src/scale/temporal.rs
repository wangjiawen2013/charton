use crate::scale::{ScaleTrait, Tick};
use time::{OffsetDateTime, Duration};

/// A scale for temporal (date/time) data using the `time` crate.
/// 
/// It maps `OffsetDateTime` values to a continuous physical range by 
/// converting time points into Unix nanosecond timestamps.
pub struct TemporalScale {
    /// The input temporal boundaries.
    domain: (OffsetDateTime, OffsetDateTime),
    /// The output visual boundaries: (start_pixel, end_pixel).
    range: (f64, f64),
}

impl TemporalScale {
    /// Creates a new `TemporalScale` with detailed English documentation.
    /// 
    /// # Arguments
    /// * `domain` - A tuple of (start_time, end_time).
    /// * `range` - A tuple of (start_pixel, end_pixel).
    pub fn new(domain: (OffsetDateTime, OffsetDateTime), range: (f64, f64)) -> Self {
        Self { domain, range }
    }

    /// Helper method to map an `OffsetDateTime` directly to a pixel coordinate.
    pub fn map_time(&self, value: OffsetDateTime) -> f64 {
        self.map(value.unix_timestamp_nanos() as f64)
    }

    /// Determines the best time interval and a label key for ticks based on the domain duration.
    /// 
    /// Returns a tuple of (step_duration, format_identifier).
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
    /// Maps a timestamp (as f64 nanoseconds) to a pixel coordinate.
    /// 
    /// The transformation is linear in terms of nanoseconds from the Unix epoch.
    fn map(&self, value: f64) -> f64 {
        let d_min = self.domain.0.unix_timestamp_nanos() as f64;
        let d_max = self.domain.1.unix_timestamp_nanos() as f64;
        let (r_min, r_max) = self.range;

        if (d_max - d_min).abs() < f64::EPSILON {
            return r_min;
        }

        let ratio = (value - d_min) / (d_max - d_min);
        r_min + ratio * (r_max - r_min)
    }

    /// Returns the domain boundaries converted to Unix nanosecond timestamps.
    fn domain(&self) -> (f64, f64) {
        (
            self.domain.0.unix_timestamp_nanos() as f64,
            self.domain.1.unix_timestamp_nanos() as f64,
        )
    }

    /// Returns the visual output range (min_pixel, max_pixel).
    fn range(&self) -> (f64, f64) {
        self.range
    }

    /// Generates human-readable temporal ticks.
    /// 
    /// This method selects an appropriate time unit (e.g., Days, Months, Years)
    /// and uses the `time` crate's macro-based formatting for high performance.
    fn ticks(&self, _count: usize) -> Vec<Tick> {
        let (start, end) = self.domain;
        let (interval, format_key) = self.get_interval_info();
        
        // Define format descriptions using literal strings to satisfy the macro's requirements.
        // This avoids the "not a macro" and "dynamic string" errors.
        let mut ticks = Vec::new();
        let mut curr = start;
        let mut iterations = 0;

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
            
            // Advance time and check for overflow
            match curr.checked_add(interval) {
                Some(next) => curr = next,
                None => break,
            }
            iterations += 1;
        }
        
        ticks
    }
}