use super::{ScaleTrait, Scale, ScaleDomain, Tick, mapper::VisualMapper};
use time::{OffsetDateTime, Duration};

/// A scale for temporal (date/time) data using the `time` crate.
/// 
/// It maps `OffsetDateTime` values to a normalized [0, 1] range by 
/// converting time points into Unix nanosecond timestamps and performing
/// linear interpolation.
/// 
/// In Charton's architecture, a `TemporalScale` can be associated with a 
/// `VisualMapper`. This allows you to encode time onto aesthetics other than 
/// position, such as mapping the age of a data point to a color gradient.
#[derive(Debug, Clone)]
pub struct TemporalScale {
    /// The input temporal boundaries (Start Time, End Time).
    /// These boundaries include any expansion padding.
    domain: (OffsetDateTime, OffsetDateTime),

    /// The optional visual mapper used to convert normalized time ratios
    /// into aesthetics like colors or sizes.
    mapper: Option<VisualMapper>,
}

impl TemporalScale {
    /// Creates a new `TemporalScale`.
    /// 
    /// # Arguments
    /// * `domain` - A tuple of (start_time, end_time). Should be expanded 
    ///               if padding is desired.
    /// * `mapper` - Optional visual logic for aesthetic mapping.
    pub fn new(domain: (OffsetDateTime, OffsetDateTime), mapper: Option<VisualMapper>) -> Self {
        Self { domain, mapper }
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
    fn scale_type(&self) -> Scale { Scale::Temporal }

    /// Transforms an absolute timestamp (as f64 nanoseconds) into a [0, 1] ratio.
    /// 
    /// By using f64, we maintain enough precision to represent individual seconds 
    /// even within a 100-year time span, avoiding the "clumping" of data points.
    fn normalize(&self, value: f64) -> f64 {
        let start_ns = self.domain.0.unix_timestamp_nanos() as f64;
        let end_ns = self.domain.1.unix_timestamp_nanos() as f64;

        let diff = end_ns - start_ns;
        
        // Safety check for zero-length domains
        if diff.abs() < 1e-9 {
            return 0.5;
        }

        // Calculation stays in f64 until the very last step
        ((value - start_ns) / diff) as f64
    }

    /// Temporal scales are continuous and do not use string-based normalization.
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

    /// For temporal scales, the logical maximum is 1.0, treating time
    /// as a continuous dimension for visual encodings.
    fn logical_max(&self) -> f64 {
        1.0
    }

    /// Returns the associated `VisualMapper` for this temporal scale.
    fn mapper(&self) -> Option<&VisualMapper> {
        self.mapper.as_ref()
    }

    /// Generates human-readable temporal ticks.
    /// 
    /// This implementation uses the `count` argument (derived from physical pixel space)
    /// to dynamically choose the most appropriate time interval (e.g., daily vs monthly)
    /// to prevent label overlapping while maintaining a consistent visual rhythm.
    fn ticks(&self, count: usize) -> Vec<Tick> {
        let (start, end) = self.domain;
        let total_duration = end - start;
        let total_seconds = total_duration.whole_seconds().abs() as f64;

        // Calculate the target density: how many seconds should each tick represent?
        // This links the mathematical scale to the physical 50px-step requirement.
        let seconds_per_tick = total_seconds / (count.max(1) as f64);
        
        // Adaptive Interval Selection:
        // Choose the smallest logical time unit that fits within the requested density.
        let (interval, format_key) = if seconds_per_tick > 365.0 * 24.0 * 3600.0 {
            (Duration::days(365), "year")
        } else if seconds_per_tick > 30.0 * 24.0 * 3600.0 {
            (Duration::days(30), "month")
        } else if seconds_per_tick > 24.0 * 3600.0 {
            (Duration::days(1), "day")
        } else if seconds_per_tick > 3600.0 {
            (Duration::hours(1), "hour")
        } else {
            (Duration::seconds(1), "second")
        };

        let mut ticks = Vec::new();
        let mut curr = start;
        let mut iterations = 0;

        // Ensure we don't enter an infinite loop and stay within the domain.
        // We use a safety margin (count * 2) to handle interval alignment issues.
        while curr <= end && iterations < count * 2 {
            ticks.push(Tick {
                value: curr.unix_timestamp_nanos() as f64,
                label: self.format_dt(curr, format_key),
            });
            
            curr = match curr.checked_add(interval) {
                Some(next) => next,
                None => break,
            };
            iterations += 1;
        }
        
        ticks
    }

    /// Returns the temporal domain as a ScaleDomain enum for guide logic.
    fn get_domain_enum(&self) -> ScaleDomain {
        ScaleDomain::Temporal(self.domain.0, self.domain.1)
    }


    /// Force-samples the temporal domain into N equidistant time points.
    /// 
    /// Unlike `ticks`, this guarantees exactly `n` points. It uses the same 
    /// adaptive formatting logic to ensure labels remain clean and appropriate 
    /// for the time span.
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

        // Determine formatting based on the step size between samples
        let seconds_per_sample = step_ns / 1e9;
        let format_key = if seconds_per_sample > 365.0 * 24.0 * 3600.0 { "year" }
            else if seconds_per_sample > 30.0 * 24.0 * 3600.0 { "month" }
            else if seconds_per_sample > 24.0 * 3600.0 { "day" }
            else if seconds_per_sample > 3600.0 { "hour" }
            else { "second" };

        (0..n).map(|i| {
            let current_ns = if i == n - 1 { end_ns } else { start_ns + i as f64 * step_ns };
            let dt = OffsetDateTime::from_unix_timestamp_nanos(current_ns as i128)
                .unwrap_or(start_dt);

            Tick {
                value: current_ns,
                label: self.format_dt(dt, format_key),
            }
        }).collect()
    }
}