pub(crate) mod cartesian;
// pub(crate) mod polar;
// pub(crate) mod time;

/// Represents the scale type for coordinate systems.
///
/// This enum defines different scaling methods that can be applied to axes
/// in various coordinate systems like cartesian, polar, or time-based plots.
#[derive(Clone, Debug, PartialEq)]
pub enum Scale {
    /// Linear scale where equal distances represent equal values.
    ///
    /// This is the most common scale where the relationship between
    /// displayed distance and value difference is proportional.
    Linear,

    /// Logarithmic scale where equal distances represent equal ratios.
    ///
    /// In this scale, each tick represents a multiplication factor rather
    /// than an addition factor. Useful for data spanning several orders of magnitude.
    Log,

    /// Discrete scale for categorical data.
    ///
    /// Used when data points are distinct categories rather than continuous values.
    /// Each category is given equal spacing regardless of any inherent numerical relationship.
    Discrete,
}
