use super::{ExplicitTick, Scale, ScaleDomain, ScaleTrait, Tick, mapper::VisualMapper};
use time::{Duration, OffsetDateTime};

/// A high-precision temporal scale mapping nanosecond timestamps to a [0, 1] visual range.
///
/// Internally stores domain boundaries as `i64` nanoseconds to align with
/// Arrow/Polars memory layouts while providing a rich API for date-time objects.
#[derive(Debug, Clone)]
pub struct TemporalScale {
    /// Domain boundaries in Unix nanoseconds (Start, End).
    domain: (i64, i64),
    /// Optional visual mapper for aesthetic encodings (color, size, etc.).
    mapper: Option<VisualMapper>,
}

/// Human-friendly time intervals for axis ticks.
const TICK_LADDER: &[(Duration, &str)] = &[
    // --- Sub-second ---
    (Duration::microseconds(1), "microsecond"),
    (Duration::microseconds(10), "microsecond"),
    (Duration::microseconds(100), "microsecond"),
    (Duration::milliseconds(1), "millisecond"),
    (Duration::milliseconds(10), "millisecond"),
    (Duration::milliseconds(100), "millisecond"),
    // --- Seconds ---
    (Duration::seconds(1), "second"),
    (Duration::seconds(5), "second"),
    (Duration::seconds(15), "second"),
    (Duration::seconds(30), "second"),
    // --- Minutes ---
    (Duration::minutes(1), "minute"),
    (Duration::minutes(5), "minute"),
    (Duration::minutes(15), "minute"),
    (Duration::minutes(30), "minute"),
    // --- Hours ---
    (Duration::hours(1), "hour"),
    (Duration::hours(3), "hour"),
    (Duration::hours(6), "hour"),
    (Duration::hours(12), "hour"),
    // --- Days & Weeks ---
    (Duration::days(1), "day"),
    (Duration::days(2), "day"),
    (Duration::days(7), "day"),
    (Duration::days(14), "day"),
    // --- Months (Average) ---
    (Duration::seconds(2629746), "month"),
    (Duration::seconds(5259492), "month"),
    (Duration::seconds(7889238), "month"),
    (Duration::seconds(15778476), "month"),
    // --- Years (Average) ---
    (Duration::seconds(31557600), "year"),
    (Duration::seconds(63115200), "year"),
    (Duration::seconds(157788000), "year"),
    (Duration::seconds(315576000), "year"),
];

impl TemporalScale {
    pub fn new(domain: (i64, i64), mapper: Option<VisualMapper>) -> Self {
        Self { domain, mapper }
    }

    /// Selects the best visual format based on the calculated density.
    fn pick_format_and_interval(seconds_per_tick: f64) -> (Duration, &'static str) {
        let best_match = TICK_LADDER
            .iter()
            .find(|(interval, _)| interval.as_seconds_f64() >= seconds_per_tick)
            .cloned();

        if let Some(found) = best_match {
            let idx = TICK_LADDER.iter().position(|x| x.0 == found.0).unwrap();
            if idx > 0 {
                let smaller = TICK_LADDER[idx - 1];
                let diff_larger = (found.0.as_seconds_f64() - seconds_per_tick).abs();
                let diff_smaller = (smaller.0.as_seconds_f64() - seconds_per_tick).abs();

                // Prefer smaller interval if it's significantly closer to target density
                if diff_smaller < diff_larger * 0.5 {
                    return smaller;
                }
            }
            return found;
        }
        (Duration::days(365 * 10), "year")
    }

    /// Snaps a timestamp to a "clean" calendar or mathematical boundary.
    fn align_to_interval(ns: i64, interval: Duration) -> i64 {
        let dt = OffsetDateTime::from_unix_timestamp_nanos(ns as i128)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
            .to_offset(time::UtcOffset::UTC);

        let days = interval.whole_days();

        if days >= 365 {
            // Yearly Alignment: Snap to the start of a year divisible by the step (e.g., 2020, 2025)
            let step_years = (days / 365).max(1) as i32;
            let aligned_year = (dt.year() / step_years) * step_years;
            dt.replace_year(aligned_year)
                .unwrap_or(dt)
                .replace_month(time::Month::January)
                .unwrap()
                .replace_day(1)
                .unwrap()
                .replace_time(time::macros::time!(00:00))
                .unix_timestamp_nanos() as i64
        } else if days >= 28 {
            // Monthly Alignment: Snap to 1st day of a month divisible by the step (e.g., Q1, Q2)
            let step_months = (interval.whole_seconds() / 2629746).max(1) as i32;
            let month0 = dt.month() as i32 - 1;
            let aligned_month0 = (month0 / step_months) * step_months;
            let aligned_month =
                time::Month::try_from((aligned_month0 + 1) as u8).unwrap_or(time::Month::January);
            dt.replace_month(aligned_month)
                .unwrap()
                .replace_day(1)
                .unwrap()
                .replace_time(time::macros::time!(00:00))
                .unix_timestamp_nanos() as i64
        } else {
            // Linear Alignment: Perfect for sub-day scales
            let interval_ns = interval.whole_nanoseconds() as i64;
            if interval_ns <= 0 {
                return ns;
            }
            ns.div_euclid(interval_ns) * interval_ns
        }
    }

    /// Formats nanoseconds into strings, with fallback for astronomical time.
    fn format_ns(&self, ns: i64, format_key: &str) -> String {
        match OffsetDateTime::from_unix_timestamp_nanos(ns as i128) {
            Ok(dt) => match format_key {
                "year" => dt.format(&time::macros::format_description!("[year]")),
                "month" => dt.format(&time::macros::format_description!("[year]-[month]")),
                "day" => dt.format(&time::macros::format_description!("[year]-[month]-[day]")),
                "hour" => dt.format(&time::macros::format_description!(
                    "[month]-[day] [hour]:[minute]"
                )),
                "minute" => dt.format(&time::macros::format_description!("[hour]:[minute]")),
                "second" => dt.format(&time::macros::format_description!(
                    "[hour]:[minute]:[second]"
                )),
                "millisecond" => dt.format(&time::macros::format_description!(
                    "[hour]:[minute]:[second].[subsecond digits:3]"
                )),
                "microsecond" => dt.format(&time::macros::format_description!(
                    "[hour]:[minute]:[second].[subsecond digits:6]"
                )),
                _ => dt.format(&time::macros::format_description!(
                    "[year]-[month]-[day] [hour]:[minute]:[second]"
                )),
            }
            .unwrap_or_else(|_| "Time Error".to_string()),
            Err(_) => {
                // Historical/Deep-time fallback: Calculate absolute year using Unix Epoch (1970) as base
                let julian_year_ns = 31_557_600.0 * 1e9;
                let absolute_year = 1970.0 + (ns as f64 / julian_year_ns);
                if absolute_year.abs() >= 1e6 {
                    format!("{:.2e} y", absolute_year)
                } else {
                    format!("{:.1} y", absolute_year)
                }
            }
        }
    }
}

impl ScaleTrait for TemporalScale {
    fn scale_type(&self) -> Scale {
        Scale::Temporal
    }

    fn normalize(&self, value: f64) -> f64 {
        let start_ns = self.domain.0 as i128;
        let diff = (self.domain.1 as i128 - start_ns) as f64;
        if diff.abs() < 1.0 {
            return 0.5;
        }
        ((value as i128 - start_ns) as f64) / diff
    }

    fn normalize_string(&self, _v: &str) -> f64 {
        f64::NAN
    }
    fn domain(&self) -> (f64, f64) {
        (self.domain.0 as f64, self.domain.1 as f64)
    }
    fn logical_max(&self) -> f64 {
        1.0
    }
    fn mapper(&self) -> Option<&VisualMapper> {
        self.mapper.as_ref()
    }

    fn suggest_ticks(&self, count: usize) -> Vec<Tick> {
        let (start, end) = self.domain;
        if start == end {
            return vec![];
        }

        // Epsilon to prevent missing the last tick due to float precision
        let end_with_epsilon = end + 1;
        let seconds_per_tick = (end - start).abs() as f64 / (1e9 * count.max(1) as f64);
        let (interval, format_key) = Self::pick_format_and_interval(seconds_per_tick);
        let interval_ns = interval.whole_nanoseconds() as i64;

        let mut ticks = Vec::new();
        let mut curr = Self::align_to_interval(start, interval);
        let mut safety_limit = 0; // Prevent infinite loops

        while curr <= end_with_epsilon && safety_limit < 1000 {
            safety_limit += 1;
            if curr >= start && curr <= end {
                ticks.push(Tick {
                    value: curr as f64,
                    label: self.format_ns(curr, format_key),
                });
            }

            let next_ns: Option<i64> = (|| {
                let dt = OffsetDateTime::from_unix_timestamp_nanos(curr as i128)
                    .ok()?
                    .to_offset(time::UtcOffset::UTC);
                match format_key {
                    "year" => {
                        let step = (interval.whole_seconds() / 31557600).max(1) as i32;
                        dt.replace_year(dt.year().checked_add(step)?)
                            .ok()?
                            .unix_timestamp_nanos()
                            .try_into()
                            .ok()
                    }
                    "month" => {
                        let step = (interval.whole_seconds() / 2629746).max(1) as i32;
                        let total_m = (dt.month() as i32 - 1) + step;
                        let new_y = dt.year() + (total_m / 12);
                        let new_m = time::Month::try_from(((total_m % 12) + 1) as u8).ok()?;
                        dt.replace_year(new_y)
                            .ok()?
                            .replace_month(new_m)
                            .ok()?
                            .replace_day(1)
                            .ok()?
                            .replace_time(time::macros::time!(00:00))
                            .unix_timestamp_nanos()
                            .try_into()
                            .ok()
                    }
                    _ => curr.checked_add(interval_ns),
                }
            })();

            match next_ns {
                Some(next) if next > curr => curr = next,
                _ => break,
            }
        }
        ticks
    }

    fn create_explicit_ticks(&self, explicit: &[ExplicitTick]) -> Vec<Tick> {
        let total_sec = (self.domain.1 - self.domain.0).abs() as f64 / 1e9;
        let (_, format_key) = Self::pick_format_and_interval(total_sec / 5.0);

        explicit
            .iter()
            .filter_map(|tick| {
                let val_ns = match tick {
                    ExplicitTick::Timestamp(ns) => *ns,
                    ExplicitTick::Temporal(dt) => dt.unix_timestamp_nanos() as i64,
                    ExplicitTick::Continuous(f) => *f as i64,
                    _ => return None,
                };

                if val_ns >= self.domain.0 && val_ns <= self.domain.1 {
                    Some(Tick {
                        value: val_ns as f64,
                        label: self.format_ns(val_ns, format_key),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn get_domain_enum(&self) -> ScaleDomain {
        ScaleDomain::Temporal(self.domain.0, self.domain.1)
    }

    fn sample_n(&self, n: usize) -> Vec<Tick> {
        if n == 0 {
            return vec![];
        }
        if n == 1 {
            return vec![Tick {
                value: self.domain.0 as f64,
                label: self.format_ns(self.domain.0, "auto"),
            }];
        }
        let step = (self.domain.1 - self.domain.0) / (n - 1) as i64;
        let (_, format_key) = Self::pick_format_and_interval(step.abs() as f64 / 1e9);
        (0..n)
            .map(|i| {
                let val = self.domain.0 + (i as i64 * step);
                Tick {
                    value: val as f64,
                    label: self.format_ns(val, format_key),
                }
            })
            .collect()
    }
}
