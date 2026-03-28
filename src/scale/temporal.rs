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
/// The engine selects the smallest interval that fits the requested pixel density.
/// Values for Months and Years use Gregorian averages (30.44 days and 365.25 days)
/// to ensure stable density calculations across leap years.
const TICK_LADDER: &[(Duration, &str)] = &[
    // --- Sub-second (Engineering & High-frequency) ---
    (Duration::microseconds(1), "microsecond"),
    (Duration::microseconds(10), "microsecond"),
    (Duration::microseconds(100), "microsecond"),
    (Duration::milliseconds(1), "millisecond"),
    (Duration::milliseconds(10), "millisecond"),
    (Duration::milliseconds(100), "millisecond"),
    // --- Seconds (Real-time tracking) ---
    (Duration::seconds(1), "second"),
    (Duration::seconds(5), "second"),
    (Duration::seconds(15), "second"),
    (Duration::seconds(30), "second"),
    // --- Minutes (Common activity spans) ---
    (Duration::minutes(1), "minute"),
    (Duration::minutes(5), "minute"),
    (Duration::minutes(15), "minute"),
    (Duration::minutes(30), "minute"),
    // --- Hours (Daily schedules) ---
    (Duration::hours(1), "hour"),
    (Duration::hours(3), "hour"),
    (Duration::hours(6), "hour"),
    (Duration::hours(12), "hour"),
    // --- Days & Weeks (Longitudinal studies) ---
    (Duration::days(1), "day"),
    (Duration::days(2), "day"),
    (Duration::days(7), "day"),
    (Duration::days(14), "day"),
    // --- Months & Quarters (Using 2,629,746s per average month) ---
    (Duration::seconds(2629746), "month"),  // 1 Month
    (Duration::seconds(5259492), "month"),  // 2 Months
    (Duration::seconds(7889238), "month"),  // 3 Months (Quarter)
    (Duration::seconds(15778476), "month"), // 6 Months
    (Duration::seconds(23667714), "month"), // 9 Months
    // --- Years & Decades (Using 31,557,600s per average year) ---
    (Duration::seconds(31557600), "year"),  // 1 Year
    (Duration::seconds(63115200), "year"),  // 2 Years
    (Duration::seconds(157788000), "year"), // 5 Years
    (Duration::seconds(315576000), "year"), // 10 Years (Decade)
];

impl TemporalScale {
    /// Creates a new temporal scale. Boundaries are inclusive i64 nanoseconds.
    pub fn new(domain: (i64, i64), mapper: Option<VisualMapper>) -> Self {
        Self { domain, mapper }
    }

    /// Selects the best visual format by finding the closest available interval.
    fn pick_format_and_interval(seconds_per_tick: f64) -> (Duration, &'static str) {
        // 1. Find the first interval that is >= our target (standard d3-style)
        let best_match = TICK_LADDER
            .iter()
            .find(|(interval, _)| interval.as_seconds_f64() >= seconds_per_tick)
            .cloned();

        // 2. If we found one, also check the previous one (smaller) to see which is closer
        // This prevents 45 days from jumping all the way to 90 days if 30 days was an option.
        if let Some(found) = best_match {
            // Find the index of our found interval
            let idx = TICK_LADDER.iter().position(|x| x.0 == found.0).unwrap();
            if idx > 0 {
                let smaller = TICK_LADDER[idx - 1];
                let diff_larger = (found.0.as_seconds_f64() - seconds_per_tick).abs();
                let diff_smaller = (smaller.0.as_seconds_f64() - seconds_per_tick).abs();

                // If the smaller interval is much closer to our target, use it
                // (We can add a bias here, e.g., only pick smaller if it doesn't crowd too much)
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

        // --- Calendar-Aware Alignment ---
        // Years and Months have variable lengths. We must snap to the 1st day/month
        // rather than using raw nanosecond division.
        if days >= 365 {
            // Yearly Scale: Snap to January 1st, 00:00:00
            dt.replace_month(time::Month::January)
                .unwrap()
                .replace_day(1)
                .unwrap()
                .replace_time(time::macros::time!(00:00))
                .unix_timestamp_nanos() as i64
        } else if days >= 28 {
            // Monthly Scale: Snap to the 1st of the month, 00:00:00
            dt.replace_day(1)
                .unwrap()
                .replace_time(time::macros::time!(00:00))
                .unix_timestamp_nanos() as i64
        } else {
            // Sub-day Scales: Linear math is perfectly fine here.
            let interval_ns = interval.whole_nanoseconds() as i64;
            if interval_ns <= 0 {
                return ns;
            }
            ns.div_euclid(interval_ns) * interval_ns
        }
    }

    /// Formats a nanosecond timestamp into a human-readable string.
    /// Optimized for the dynamic range of the extended TICK_LADDER.
    fn format_ns(&self, ns: i64, format_key: &str) -> String {
        match OffsetDateTime::from_unix_timestamp_nanos(ns as i128) {
            Ok(dt) => {
                match format_key {
                    // --- Macro Scales (Full Date Context) ---
                    "year" => dt.format(&time::macros::format_description!("[year]")),
                    "month" => dt.format(&time::macros::format_description!("[year]-[month]")),
                    "day" => dt.format(&time::macros::format_description!("[year]-[month]-[day]")),

                    // --- Micro Scales (Intra-day context) ---
                    // Note: We include month-day for hours to provide "Safety Context"
                    // when a chart spans across midnight.
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
                .unwrap_or_else(|e| format!("Data error: <TimeFormat {}>", e))
            }
            Err(_) => {
                // Astronomical or deep-time fallback for timestamps outside
                // the range of standard Gregorian calendars (approx. +/- 10^9 years).
                // We use the Julian year constant (365.25 days).
                let years = (ns as f64) / (31_557_600.0 * 1e9);

                let abs_years = years.abs();

                if abs_years >= 1e6 {
                    // Use scientific notation for millions of years and beyond.
                    // e.g., "4.54e9 y" (Age of Earth)
                    format!("{:.2e} y", years)
                } else if abs_years >= 1.0 {
                    // Use one decimal place for historical scales.
                    // e.g., "2000.5 y"
                    format!("{:.1} y", years)
                } else {
                    // For sub-year scales that still failed OffsetDateTime
                    // (extremely rare but possible in edge cases).
                    format!("{:.4} y", years)
                }
            }
        }
    }
}

impl ScaleTrait for TemporalScale {
    fn scale_type(&self) -> Scale {
        Scale::Temporal
    }

    /// Transforms a nanosecond value (as f64) to a [0, 1] relative position.
    /// Uses i128 for the intermediate subtraction to prevent precision loss
    /// when zooming into micro-windows of a distant timestamp.
    fn normalize(&self, value: f64) -> f64 {
        let start_ns = self.domain.0 as i128;
        let diff = (self.domain.1 as i128 - start_ns) as f64;
        if diff.abs() < 1.0 {
            return 0.5;
        } // Avoid division by zero for identical boundaries
        ((value as i128 - start_ns) as f64) / diff
    }

    /// Unused for temporal scales as they are numeric-based.
    fn normalize_string(&self, _value: &str) -> f64 {
        0.0
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

    /// Generates human-friendly, aligned ticks (e.g., 12:00, 13:00) based on target density.
    fn suggest_ticks(&self, count: usize) -> Vec<Tick> {
        let (start, end) = self.domain;
        if start == end {
            return vec![];
        }

        let seconds_per_tick = (end - start).abs() as f64 / (1e9 * count.max(1) as f64);
        let (interval, format_key) = Self::pick_format_and_interval(seconds_per_tick);
        let interval_ns = interval.whole_nanoseconds() as i64;

        let mut ticks = Vec::new();
        let mut curr = Self::align_to_interval(start, interval);

        while curr <= end {
            if curr >= start {
                ticks.push(Tick {
                    value: curr as f64,
                    label: self.format_ns(curr, format_key),
                });
            }

            // --- Calendar-Aware Stepping ---
            let next_ns: Option<i64> = (|| {
                let dt = OffsetDateTime::from_unix_timestamp_nanos(curr as i128)
                    .ok()?
                    .to_offset(time::UtcOffset::UTC);

                match format_key {
                    "year" => {
                        // Use 31,557,600 (Average Year) to calculate the step jump
                        let step_years = (interval.whole_seconds() / 31557600) as i32;
                        dt.replace_year(dt.year() + step_years.max(1))
                            .ok()?
                            .unix_timestamp_nanos()
                            .try_into()
                            .ok()
                    }
                    "month" => {
                        // Use 2,629,746 (Average Month) to calculate the step jump
                        let step_months = (interval.whole_seconds() / 2629746) as i32;
                        let total_months = (dt.month() as i32 - 1) + step_months.max(1);

                        let new_year = dt.year() + (total_months / 12);
                        let month_num = (total_months % 12) + 1;
                        let new_month = time::Month::try_from(month_num as u8).ok()?;

                        dt.replace_year(new_year)
                            .ok()?
                            .replace_month(new_month)
                            .ok()?
                            .replace_day(1)
                            .ok()?
                            .replace_time(time::macros::time!(00:00))
                            .unix_timestamp_nanos()
                            .try_into()
                            .ok()
                    }
                    _ => {
                        // For days and below, linear addition is safe
                        curr.checked_add(interval_ns)
                    }
                }
            })();

            match next_ns {
                Some(next) if next > curr => curr = next,
                _ => break,
            }
        }

        ticks
    }

    /// Creates ticks from user-provided explicit values (Timestamps, Dates, or Nanos).
    fn create_explicit_ticks(&self, explicit: &[ExplicitTick]) -> Vec<Tick> {
        // Default to a 5-tick density heuristic for determining format
        let total_sec = (self.domain.1 - self.domain.0).abs() as f64 / 1e9;
        let (_, format_key) = Self::pick_format_and_interval(total_sec / 5.0);

        explicit
            .iter()
            .filter_map(|tick| {
                let val_ns = match tick {
                    ExplicitTick::Timestamp(ns) => *ns,
                    ExplicitTick::Temporal(dt) => dt.unix_timestamp_nanos() as i64,
                    ExplicitTick::Continuous(f) => *f as i64, // Coerce numeric f64 to nanos
                    _ => return None,
                };

                // Only include ticks within the current visible domain
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

    /// Returns the current domain as a ScaleDomain enum.
    /// Since we store raw nanoseconds, this is now a zero-risk operation.
    fn get_domain_enum(&self) -> ScaleDomain {
        ScaleDomain::Temporal(self.domain.0, self.domain.1)
    }

    /// Evenly samples N points across the domain, ignoring interval alignment.
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
