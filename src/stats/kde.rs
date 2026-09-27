//! Kernel density estimation for one-dimensional data.
//!
//! Density estimation answers a simple question: given a list of observed
//! values, how crowded is the data around an arbitrary point? Charton uses
//! the answer in two places. The density transform turns the estimate into a
//! curve, and the point layouts use it to decide how far a marker may drift
//! from its category center.
//!
//! The implementation is deliberately small and has no external dependencies.
//! Only one-dimensional data is supported, which is all a chart needs. Every
//! calculation is done in `f64` so that the same estimate serves both the
//! statistical transform and the placement of individual marks.

use std::f64::consts::PI;

/// Shape of the smoothing bump placed on every observation.
///
/// The kernel decides how a single observation spreads its influence to the
/// neighbourhood. Every supported kernel is symmetric and integrates to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelType {
    /// Gaussian bell curve. It produces the smoothest curves and is the default.
    Normal,
    /// Parabolic bump with compact support. It is the statistically optimal
    /// choice when the estimate is judged by its mean squared error.
    Epanechnikov,
    /// Rectangular bump with compact support. It behaves like a moving average.
    Uniform,
}

/// Rule that decides how wide the smoothing bump should be.
///
/// A wide bandwidth makes a smooth curve that hides small bumps, while a
/// narrow bandwidth follows the data more closely at the cost of a noisier
/// result.
#[derive(Debug, Clone, Copy)]
pub enum BandwidthType {
    /// Scott's rule of thumb, based on the spread of the data.
    Scott,
    /// Silverman's rule of thumb, based on the smaller of the spread and the
    /// interquartile range. It is more robust to outliers than Scott's rule.
    Silverman,
    /// A caller supplied width, expressed in the same units as the data.
    Fixed(f64),
}

impl KernelType {
    const fn as_str(&self) -> &'static str {
        match self {
            KernelType::Normal => "Normal",
            KernelType::Epanechnikov => "Epanechnikov",
            KernelType::Uniform => "Uniform",
        }
    }
}

impl std::fmt::Display for KernelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl BandwidthType {
    const fn as_str(&self) -> &'static str {
        match self {
            BandwidthType::Scott => "Scott",
            BandwidthType::Silverman => "Silverman",
            BandwidthType::Fixed(_) => "Fixed",
        }
    }
}

impl std::fmt::Display for BandwidthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BandwidthType::Fixed(value) => write!(f, "Fixed({})", value),
            _ => write!(f, "{}", self.as_str()),
        }
    }
}

/// Sample variance with the usual `n - 1` denominator.
///
/// A single observation has no spread, so the result is zero instead of a
/// division by zero.
fn variance(data: &[f64]) -> f64 {
    let n = data.len();
    if n < 2 {
        return 0.0;
    }
    let mean = data.iter().sum::<f64>() / n as f64;
    let squares = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>();
    squares / (n - 1) as f64
}

/// Quantile using the same interpolation as the common plotting defaults.
fn quantile(data: &[f64], tau: f64) -> f64 {
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    // The +1 keeps the tails inside the array; clamping protects tiny samples.
    let index = (tau * (n + 1) as f64).round() as isize;
    let index = index.clamp(1, n as isize) as usize;
    sorted[index - 1]
}

/// Distance between the first and third quartile, scaled to match the spread.
fn interquartile_range(data: &[f64]) -> f64 {
    quantile(data, 0.75) - quantile(data, 0.25)
}

/// A prepared one-dimensional kernel density estimate.
pub(crate) struct Kde {
    observations: Vec<f64>,
    bandwidth: BandwidthType,
    kernel: KernelType,
}

impl Kde {
    pub(crate) const fn new(
        observations: Vec<f64>,
        bandwidth: BandwidthType,
        kernel: KernelType,
    ) -> Self {
        Self {
            observations,
            bandwidth,
            kernel,
        }
    }

    /// Value of the chosen smoothing bump at a normalized distance `u`.
    fn kernel_value(&self, u: f64) -> f64 {
        match self.kernel {
            KernelType::Normal => (-0.5 * u * u).exp() / (2.0 * PI).sqrt(),
            KernelType::Epanechnikov => {
                let term = 1.0 - u * u;
                if term > 0.0 { 0.75 * term } else { 0.0 }
            }
            KernelType::Uniform => {
                if u.abs() <= 1.0 {
                    0.5
                } else {
                    0.0
                }
            }
        }
    }

    /// A positive measure of how far the data reaches.
    ///
    /// The standard deviation is the natural choice. When the data has no
    /// spread the magnitude of the values is used instead, and finally one,
    /// so that even a fully constant column still has a usable width.
    fn data_scale(&self) -> f64 {
        let sd = variance(&self.observations).sqrt();
        if sd > 0.0 {
            return sd;
        }
        let magnitude = self
            .observations
            .iter()
            .map(|v| v.abs())
            .fold(0.0_f64, f64::max);
        if magnitude > 0.0 { magnitude } else { 1.0 }
    }

    /// Width of the smoothing bump in data units.
    ///
    /// The rule based widths can collapse to zero when the data has a narrow
    /// interquartile range or no spread at all. Falling back to the data scale
    /// keeps the estimate smooth, and a tiny positive value keeps the
    /// arithmetic finite so a constant column produces a narrow spike instead
    /// of `NaN`.
    fn bandwidth_value(&self) -> f64 {
        let n = self.observations.len() as f64;
        let damping = n.powf(-0.2);

        let raw = match self.bandwidth {
            BandwidthType::Fixed(h) => h,
            BandwidthType::Scott => 1.06 * self.data_scale() * damping,
            BandwidthType::Silverman => {
                let sd = variance(&self.observations).sqrt();
                let iqr = interquartile_range(&self.observations) / 1.349;
                let spread = if sd.min(iqr) > 0.0 {
                    sd.min(iqr)
                } else {
                    self.data_scale()
                };
                0.9 * spread * damping
            }
        };

        if raw.is_finite() && raw > 0.0 {
            raw
        } else {
            self.data_scale() * 1e-9
        }
    }

    /// Estimated probability density at every requested point.
    pub(crate) fn pdf(&self, points: &[f64]) -> Vec<f64> {
        let n = self.observations.len();
        if n == 0 {
            return vec![0.0; points.len()];
        }

        let h = self.bandwidth_value();
        let prefactor = 1.0 / (n as f64 * h);

        points
            .iter()
            .map(|&x| {
                let sum: f64 = self
                    .observations
                    .iter()
                    .map(|&xi| self.kernel_value((x - xi) / h))
                    .sum();
                sum * prefactor
            })
            .collect()
    }

    /// Estimated cumulative distribution at every requested point.
    ///
    /// The points are expected in ascending order. The curve is the running
    /// sum of the density, rescaled so that it reaches one at the end.
    pub(crate) fn cdf(&self, points: &[f64]) -> Vec<f64> {
        let density = self.pdf(points);
        let mut running = 0.0;
        let mut out: Vec<f64> = Vec::with_capacity(density.len());
        for value in density {
            running += value;
            out.push(running);
        }
        if running > 0.0 {
            for value in &mut out {
                *value /= running;
            }
        }
        out
    }
}

/// Number of samples used to scan the data range when building a profile.
const PROFILE_GRID: usize = 256;

/// Estimates how crowded the data is at each of its own values.
///
/// The estimate is built on a small grid over the data range and then read
/// back at the exact observation positions. This keeps the work proportional
/// to the number of points instead of their square. The returned values are
/// scaled so that the most crowded point has a density of one, which is the
/// form a layout needs when turning density into an offset.
pub(crate) fn density_profile(data: &[f64], bandwidth: &BandwidthType) -> Vec<f64> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
    for &value in data {
        min = min.min(value);
        max = max.max(value);
    }

    // A group where every value is the same has a flat density by definition.
    if (max - min) < 1e-12 {
        return vec![1.0; n];
    }

    let kde = Kde::new(data.to_vec(), *bandwidth, KernelType::Normal);
    let grid: Vec<f64> = (0..PROFILE_GRID)
        .map(|i| min + (max - min) * i as f64 / (PROFILE_GRID - 1) as f64)
        .collect();
    let grid_density = kde.pdf(&grid);

    let mut out: Vec<f64> = data
        .iter()
        .map(|&value| {
            let position = (value - min) / (max - min) * (PROFILE_GRID - 1) as f64;
            let index = (position.floor() as usize).min(PROFILE_GRID - 2);
            let fraction = position - index as f64;
            grid_density[index] * (1.0 - fraction) + grid_density[index + 1] * fraction
        })
        .collect();

    let peak = out.iter().copied().fold(0.0_f64, f64::max);
    if peak > 0.0 {
        for value in &mut out {
            *value /= peak;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: [f64; 5] = [1.0, 1.5, 2.0, 2.5, 3.0];

    #[test]
    fn bandwidth_rules_match_known_values() {
        let scott = Kde::new(DATA.to_vec(), BandwidthType::Scott, KernelType::Normal);
        assert!((scott.bandwidth_value() - 0.607_36).abs() < 1e-4);

        let silverman = Kde::new(DATA.to_vec(), BandwidthType::Silverman, KernelType::Normal);
        assert!((silverman.bandwidth_value() - 0.515_68).abs() < 1e-4);
    }

    #[test]
    fn density_integrates_to_one_approximately() {
        let kde = Kde::new(DATA.to_vec(), BandwidthType::Scott, KernelType::Normal);
        let points: Vec<f64> = (0..200).map(|i| -1.0 + i as f64 * 0.05).collect();
        let area: f64 = kde.pdf(&points).iter().sum::<f64>() * 0.05;
        assert!((area - 1.0).abs() < 0.05);
    }

    #[test]
    fn cumulative_distribution_rises_to_one() {
        let kde = Kde::new(DATA.to_vec(), BandwidthType::Scott, KernelType::Normal);
        let points: Vec<f64> = (0..100).map(|i| -1.0 + i as f64 * 0.06).collect();
        let cdf = kde.cdf(&points);
        assert!(cdf.first().unwrap() < cdf.last().unwrap());
        assert!((cdf.last().unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn constant_data_does_not_produce_nan() {
        let kde = Kde::new(vec![2.0; 7], BandwidthType::Scott, KernelType::Normal);
        assert!(kde.pdf(&[2.0, 2.1]).iter().all(|v| v.is_finite()));
    }

    #[test]
    fn density_profile_is_flat_for_constant_data() {
        let profile = density_profile(&[4.0; 6], &BandwidthType::Silverman);
        assert_eq!(profile, vec![1.0; 6]);
    }

    #[test]
    fn density_profile_peaks_where_values_cluster() {
        // A tight crowd of values plus a small group far away.
        let mut data = vec![0.0; 50];
        data.extend([10.0; 5]);
        let profile = density_profile(&data, &BandwidthType::Scott);
        let cluster = profile[0];
        let sparse = profile[50];
        assert!(cluster > sparse);
    }
}
