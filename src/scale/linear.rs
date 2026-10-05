use super::{ExplicitTick, Scale, ScaleDomain, ScaleTrait, Tick, mapper::VisualMapper};

/// A scale that maps a continuous data domain to a normalized [0, 1] range.
///
/// The `LinearScale` is the workhorse of quantitative visualization. It performs
/// the mathematical transformation for positional channels (X, Y) and provides
/// the basis for continuous visual mappings (Color, Size).
///
/// In Charton's architecture, a `LinearScale` is often shared via `Arc` across
/// multiple layers to ensure they all use the same data-to-visual mapping.
///
/// The domain may run either way: a **reversed** domain (`min > max`) flips the
/// axis, which is how a rank axis puts rank 1 at the top. Padding and ticks
/// still work — they are applied to the numeric span, not to the direction.
#[derive(Debug, Clone)]
pub struct LinearScale {
    /// The input data boundaries: (min_value, max_value).
    /// These values typically include expansion padding calculated during training.
    domain: (f64, f64),

    /// The optional visual mapper that defines how the normalized [0, 1] value
    /// is converted into concrete aesthetics like colors or physical sizes.
    mapper: Option<VisualMapper>,
}

impl LinearScale {
    /// Creates a new `LinearScale` with a specified domain and an optional visual mapper.
    ///
    /// # Arguments
    /// * `domain` - A tuple of (min, max) representing the expanded data range.
    /// * `mapper` - An optional `VisualMapper` for non-positional aesthetics.
    pub const fn new(domain: (f64, f64), mapper: Option<VisualMapper>) -> Self {
        Self { domain, mapper }
    }

    /// Calculates a "nice" step size for axis ticks (e.g., 0.1, 0.2, 0.5, 1.0).
    ///
    /// This ensures that the intervals between ticks are intuitive for human readers.
    /// It uses the range of the domain and a target tick count to find the
    /// optimal power-of-ten interval.
    fn calculate_nice_step(&self, count: usize) -> f64 {
        let (min, max) = self.domain;
        // `abs` so a reversed domain (min > max) still gets a sensible step.
        let range = (max - min).abs();

        // Safety check for single-point domains or identical boundaries.
        if range.abs() < 1e-12 {
            return 1.0;
        }

        let rough_step = range / (count.max(2) as f64);

        // Magnitude (power of 10) of the rough step.
        let exp = 10f64.powf(rough_step.log10().floor());

        // Normalize the step to the [1, 10] range to pick the best "nice" factor.
        let f = rough_step / exp;

        let nice = if f < 1.5 {
            1.0
        } else if f < 3.0 {
            2.0
        } else if f < 7.0 {
            5.0
        } else {
            10.0
        };

        nice * exp
    }
}

impl ScaleTrait for LinearScale {
    fn scale_type(&self) -> Scale {
        Scale::Linear
    }

    /// Maps a raw data value to a normalized [0.0, 1.0] float.
    ///
    /// Formula: `(value - min) / (max - min)`.
    /// If the value is within the expansion padding, the result will be
    /// slightly inside the [0, 1] range.
    fn normalize(&self, value: f64) -> f64 {
        let (d_min, d_max) = self.domain;
        let diff = d_max - d_min;

        if diff.abs() < f64::EPSILON {
            return 0.5; // Default to center for zero-width domains.
        }

        (value - d_min) / diff
    }

    /// Continuous linear scales return a fallback for categorical string inputs.
    fn normalize_string(&self, _value: &str) -> f64 {
        f64::NAN
    }

    /// Returns the data boundaries used by this scale.
    fn domain(&self) -> (f64, f64) {
        self.domain
    }

    /// For continuous scales, the logical maximum is always 1.0,
    /// representing 100% of the mapping range.
    fn logical_max(&self) -> f64 {
        1.0
    }

    /// Returns a reference to the internal `VisualMapper`.
    ///
    /// This is used by marks (e.g., a bubble) to determine their specific
    /// visual property (e.g., color) after the data value has been normalized.
    fn mapper(&self) -> Option<&VisualMapper> {
        self.mapper.as_ref()
    }

    /// Generates human-readable tick marks based on the domain.
    ///
    /// This method automatically adjusts the precision of the string labels
    /// based on the magnitude of the calculated nice step.
    fn suggest_ticks(&self, count: usize) -> Vec<Tick> {
        let (min, max) = self.domain;
        let step = self.calculate_nice_step(count);
        let tolerance = step * 1e-9;

        // Generate the values ascending over the numeric range; the axis places
        // them with `normalize`, which already honours a reversed domain.
        let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
        let start = (lo / step).ceil() * step;
        let mut values = Vec::new();
        let mut curr = start;

        let mut iterations = 0;
        while curr <= hi + tolerance && iterations < count * 2 {
            let clean_val = if curr.abs() < 1e-12 { 0.0 } else { curr };
            values.push(clean_val);

            curr += step;
            iterations += 1;
        }

        // Ensure consistent axis-wide formatting (automatic scientific notation)
        super::format_ticks(&values)
    }

    /// Transforms user-defined explicit ticks into renderable Tick objects.
    ///
    /// This implementation performs three steps:
    /// 1. Filters out non-continuous variants from the input.
    /// 2. Validates that the values fall within the current [min, max] domain.
    /// 3. Formats the valid values into strings using the shared formatting logic.
    fn create_explicit_ticks(&self, explicit: &[ExplicitTick]) -> Vec<Tick> {
        let (min, max) = self.domain;
        // A reversed domain still has the same numeric span; only the direction
        // of the axis changes.
        let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
        // Pre-calculate tolerance once to avoid repeating in the loop
        let range = hi - lo;
        let tolerance = if range < f64::EPSILON {
            1e-10
        } else {
            range * 1e-10
        };

        let mut type_mismatch = 0;
        let mut out_of_domain = 0;

        // Plain numeric ticks are formatted automatically after the loop; ticks
        // that carry a label are kept with that label untouched.
        let mut continuous_values: Vec<f64> = Vec::new();
        let mut labeled_ticks: Vec<Tick> = Vec::new();

        for tick in explicit {
            match tick {
                ExplicitTick::Continuous(val) => {
                    // Logic: Only allow values within the numeric range, in either
                    // direction.
                    if *val >= lo - tolerance && *val <= hi + tolerance {
                        // Clean up near-zero values for cleaner labels
                        continuous_values.push(if val.abs() < 1e-12 { 0.0 } else { *val });
                    } else {
                        out_of_domain += 1;
                    }
                }
                ExplicitTick::Labeled(val, label) => {
                    // A labelled tick is a normal position plus a custom name.
                    if *val >= lo - tolerance && *val <= hi + tolerance {
                        labeled_ticks.push(Tick {
                            value: *val,
                            label: label.clone(),
                        });
                    } else {
                        out_of_domain += 1;
                    }
                }
                // Count mismatches for a single bulk warning later
                _ => {
                    type_mismatch += 1;
                }
            }
        }

        // High-Performance Logging: Bulk report issues after the hot loop
        if type_mismatch > 0 || out_of_domain > 0 {
            eprintln!(
                "LinearScale: Filtered {} ticks ({} type mismatch, {} out of domain [{}, {}]).",
                type_mismatch + out_of_domain,
                type_mismatch,
                out_of_domain,
                min,
                max
            );
        }

        // Delegate to the shared formatting logic for the numeric ticks, then
        // add the hand-labelled ones and keep everything in axis order.
        let mut ticks = super::format_ticks(&continuous_values);
        ticks.extend(labeled_ticks);
        ticks.sort_by(|a, b| {
            a.value
                .partial_cmp(&b.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ticks
    }

    /// Returns the domain specification for chart guide and legend logic.
    fn get_domain_enum(&self) -> ScaleDomain {
        ScaleDomain::Continuous(self.domain.0, self.domain.1)
    }

    /// Equidistant sampling used for legends that require fixed number legends.
    ///
    /// Unlike `ticks`, this guarantees exactly `n` points, even if the
    /// values are not "pretty" decimals.
    fn sample_n(&self, n: usize) -> Vec<Tick> {
        let (min, max) = self.domain;

        if n == 0 {
            return Vec::new();
        }
        if n == 1 {
            return super::format_ticks(&[min]);
        }

        let step = (max - min) / (n - 1) as f64;
        let values: Vec<f64> = (0..n)
            .map(|i| {
                let val = if i == n - 1 {
                    max
                } else {
                    min + i as f64 * step
                };
                if val.abs() < 1e-12 { 0.0 } else { val }
            })
            .collect();

        super::format_ticks(&values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reversed domain (min > max) maps low values high and still produces
    /// usable ticks — this is what lets a rank axis put rank 1 at the top.
    #[test]
    fn reversed_domain_normalizes_and_ticks() {
        let scale = LinearScale::new((7.5, 0.5), None);

        assert!(scale.normalize(1.0) > 0.8, "rank 1 should sit near the top");
        assert!(
            scale.normalize(7.0) < 0.2,
            "rank 7 should sit near the bottom"
        );

        let ticks = scale.suggest_ticks(7);
        assert!(!ticks.is_empty(), "a reversed domain must still tick");
        assert!(
            ticks.iter().all(|t| (0.5..=7.5).contains(&t.value)),
            "ticks must stay inside the numeric range"
        );
    }

    /// An ascending domain must be untouched by the reversal support.
    #[test]
    fn ascending_domain_still_normalizes_upward() {
        let scale = LinearScale::new((0.0, 10.0), None);
        assert!((scale.normalize(2.0) - 0.2).abs() < 1e-12);
        assert!((scale.normalize(8.0) - 0.8).abs() < 1e-12);
    }
}
