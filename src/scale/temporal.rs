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

    /// Helper method to transform an `OffsetDateTime` directly to a normalized [0, 1] value.
    pub fn normalize_time(&self, value: OffsetDateTime) -> f32 {
        self.normalize(value.unix_timestamp_nanos() as f32)
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

    /// Transforms a timestamp (as f32 nanoseconds) into a normalized [0, 1] ratio.
    /// 
    /// The transformation is linear based on the elapsed nanoseconds from the domain start.
    fn normalize(&self, value: f32) -> f32 {
        let d_min = self.domain.0.unix_timestamp_nanos() as f32;
        let d_max = self.domain.1.unix_timestamp_nanos() as f32;

        let diff = d_max - d_min;
        if diff.abs() < f32::EPSILON {
            // Default to center if the time domain is a single point
            return 0.5;
        }

        (value - d_min) / diff
    }

    /// Temporal scales are continuous and do not use string-based normalization.
    fn normalize_string(&self, _value: &str) -> f32 {
        0.0
    }

    /// Returns the domain boundaries converted to Unix nanosecond timestamps (f32).
    fn domain(&self) -> (f32, f32) {
        (
            self.domain.0.unix_timestamp_nanos() as f32,
            self.domain.1.unix_timestamp_nanos() as f32,
        )
    }

    /// For temporal scales, the logical maximum is 1.0, treating time
    /// as a continuous dimension for visual encodings.
    fn logical_max(&self) -> f32 {
        1.0
    }

    /// Returns the associated `VisualMapper` for this temporal scale.
    fn mapper(&self) -> Option<&VisualMapper> {
        self.mapper.as_ref()
    }

    /// Generates human-readable temporal ticks.
    /// 
    /// Selects an appropriate time unit (e.g., Days, Months, Years)
    /// based on the domain duration.
    fn ticks(&self, _count: usize) -> Vec<Tick> {
        let (start, end) = self.domain;
        let (interval, format_key) = self.get_interval_info();
        
        let mut ticks = Vec::new();
        let mut curr = start;
        let mut iterations = 0;

        while curr <= end && iterations < 50 {
            let label = self.format_dt(curr, format_key);

            ticks.push(Tick {
                value: curr.unix_timestamp_nanos() as f32,
                label,
            });
            
            match curr.checked_add(interval) {
                Some(next) => curr = next,
                None => break,
            }
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
    /// Useful for generating colorbars or legends that represent a timeline
    /// with uniform visual density.
    fn sample_n(&self, n: usize) -> Vec<Tick> {
        let (start_dt, end_dt) = self.domain;
        
        if n == 0 { return Vec::new(); }
        if n == 1 {
            let (_, format_key) = self.get_interval_info();
            return vec![Tick {
                value: start_dt.unix_timestamp_nanos() as f32,
                label: self.format_dt(start_dt, format_key),
            }];
        }

        let start_ns = start_dt.unix_timestamp_nanos() as f32;
        let end_ns = end_dt.unix_timestamp_nanos() as f32;
        let step_ns = (end_ns - start_ns) / (n - 1) as f32;

        let (_, format_key) = self.get_interval_info();

        (0..n).map(|i| {
            let current_ns = if i == n - 1 { end_ns } else { start_ns + i as f32 * step_ns };
            
            let dt = OffsetDateTime::from_unix_timestamp_nanos(current_ns as i128)
                .unwrap_or(start_dt);

            Tick {
                value: current_ns,
                label: self.format_dt(dt, format_key),
            }
        }).collect()
    }
}