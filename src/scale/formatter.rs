//! User-configurable formatting of tick and legend labels.
//!
//! By default Charton derives labels from the scale itself (pretty numbers,
//! scientific notation, category names). That is a good default, but production
//! charts regularly need more: currency prefixes, compact notation, a fixed
//! number of decimals, thousands separators, or an entirely custom string.
//!
//! [`LabelFormat`] is the builder that covers those cases.
//!
//! # How a format is applied
//!
//! A format is attached to a scale by wrapping it in [`FormattedScale`]. The
//! wrapper passes every calculation through to the wrapped scale and changes
//! only [`Tick::label`]. All ticks come from `suggest_ticks`,
//! `create_explicit_ticks` or `sample_n`, so changing those three methods
//! covers every axis, grid line, legend and colour bar.
//!
//! This matters because the layout engine measures labels to reserve axis space
//! and to wrap legend entries. If the measured text and the drawn text came from
//! different places, the reserved space would not match what is on screen.
//!
//! A scale that has a format reports it through [`ScaleTrait::label_formatter`],
//! so callers can tell that the labels are already final.

use super::mapper::VisualMapper;
use super::{ExplicitTick, Scale, ScaleDomain, ScaleTrait, Tick};
use std::sync::Arc;

/// A function that turns a numeric value into display text.
pub type NumericLabelFn = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// A function that turns a categorical (or pre-formatted) label into display text.
pub type TextLabelFn = Arc<dyn Fn(&str) -> String + Send + Sync>;

/// How large (or small) magnitudes are shortened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abbreviation {
    /// Compact notation, as used by financial and news charts:
    ///
    /// | Magnitude | Suffix | Example |
    /// | --- | --- | --- |
    /// | 10^12 | `T` | `2T` |
    /// | 10^9 | `B` | `2B` |
    /// | 10^6 | `M` | `2M` |
    /// | 10^3 | `k` | `2k` |
    /// | 10^-3 | `m` | `2m` |
    /// | 10^-6 | `µ` | `2µ` |
    /// | 10^-9 | `n` | `2n` |
    /// | 10^-12 | `p` | `2p` |
    ///
    /// Note that 10^9 is `B` (billion), not the SI prefix `G` (giga).
    Compact,
}

/// A composable formatter for axis tick labels and legend entry labels.
///
/// A `LabelFormat` is a plain value (cheap to clone, `Send + Sync`) attached to a
/// chart through `with_x_label_format`, `with_y_label_format` or
/// `with_legend_label_format`.
///
/// # Precedence of the numeric options
///
/// From highest to lowest:
///
/// 1. [`Self::with_numeric_formatter`] / [`Self::with_text_formatter`] -- a
///    closure produces the core text verbatim. A numeric closure receives the
///    value *after* [`Self::with_multiplier`] has been applied, so the two compose.
/// 2. Precision, abbreviation, separators and multiplier -- the number is rendered
///    by this type from its value.
/// 3. Nothing numeric is set -- the label already produced by the scale is kept.
///
/// This last rule is what makes a decoration-only format safe: setting just a
/// prefix or suffix does **not** silently replace Charton's automatic number
/// formatting (pretty decimals, scientific notation) with a plain `Display`.
///
/// [`Self::with_prefix`] and [`Self::with_suffix`] are always applied around the
/// core text, whichever branch produced it.
///
/// # Example
///
/// ```rust,ignore
/// use charton::prelude::*;
///
/// // 0, 20_000_000_000, ... -> "$0", "$20B", ...
/// let money = LabelFormat::new()
///     .with_prefix("$")
///     .with_compact_notation()
///     .with_precision(0);
/// ```
#[derive(Clone)]
pub struct LabelFormat {
    /// Text placed before the value, e.g. a currency symbol.
    prefix: String,
    /// Text placed after the value, e.g. a unit.
    suffix: String,
    /// Fixed number of decimal places. `None` keeps the automatic precision.
    precision: Option<usize>,
    /// Whether trailing decimal zeros are removed (`20.0` -> `20`).
    trim_zeros: bool,
    /// Character inserted every three digits of the integer part.
    thousands_separator: Option<char>,
    /// Factor applied to the value before formatting, converting a data unit
    /// into a display unit (`1e-9` shows nanoseconds as seconds).
    multiplier: f64,
    /// Strategy for shortening large or small magnitudes.
    abbreviation: Option<Abbreviation>,
    /// Fully custom numeric renderer. When set, it overrides precision,
    /// multiplier, abbreviation and the thousands separator.
    numeric: Option<NumericLabelFn>,
    /// Fully custom renderer for categorical labels.
    text: Option<TextLabelFn>,
}

impl std::fmt::Debug for LabelFormat {
    /// Closures cannot be printed, so `numeric` and `text` are reported as
    /// present/absent rather than as values.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LabelFormat")
            .field("prefix", &self.prefix)
            .field("suffix", &self.suffix)
            .field("precision", &self.precision)
            .field("trim_zeros", &self.trim_zeros)
            .field("thousands_separator", &self.thousands_separator)
            .field("multiplier", &self.multiplier)
            .field("abbreviation", &self.abbreviation)
            .field("numeric", &self.numeric.is_some())
            .field("text", &self.text.is_some())
            .finish()
    }
}

impl Default for LabelFormat {
    fn default() -> Self {
        Self {
            prefix: String::new(),
            suffix: String::new(),
            precision: None,
            trim_zeros: true,
            thousands_separator: None,
            multiplier: 1.0,
            abbreviation: None,
            numeric: None,
            text: None,
        }
    }
}

impl LabelFormat {
    /// Creates an empty format. On its own it is a no-op.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // --- Text decoration ---

    /// Prepends a fixed string, e.g. a currency symbol.
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Appends a fixed string, e.g. a unit.
    #[must_use]
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    // --- Numeric rendering ---

    /// Fixes the number of decimal places. Overrides the automatic precision.
    #[must_use]
    pub const fn with_precision(mut self, precision: usize) -> Self {
        self.precision = Some(precision);
        self
    }

    /// Whether trailing decimal zeros are removed (`20.0 -> "20"`). Defaults to `true`.
    #[must_use]
    pub const fn with_trim_zeros(mut self, trim: bool) -> Self {
        self.trim_zeros = trim;
        self
    }

    /// Groups the integer part every three digits, e.g. `1234567 -> "1,234,567"`.
    #[must_use]
    pub const fn with_thousands_separator(mut self, separator: char) -> Self {
        self.thousands_separator = Some(separator);
        self
    }

    /// Multiplies every value before formatting, converting a data unit into a
    /// display unit (e.g. `with_multiplier(1e-9)` to show nanoseconds as seconds).
    ///
    /// A custom numeric formatter also receives the multiplied value, so the two
    /// options compose rather than shadow one another.
    #[must_use]
    pub const fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Enables an abbreviation strategy for large/small magnitudes.
    #[must_use]
    pub const fn with_abbreviation(mut self, abbreviation: Abbreviation) -> Self {
        self.abbreviation = Some(abbreviation);
        self
    }

    /// Shorthand for [`Abbreviation::Compact`].
    #[must_use]
    pub const fn with_compact_notation(self) -> Self {
        self.with_abbreviation(Abbreviation::Compact)
    }

    // --- Fully custom ---

    /// Replaces the numeric rendering entirely. `prefix`/`suffix` still apply.
    ///
    /// The closure receives the value after [`Self::with_multiplier`] has been
    /// applied. When set, this takes precedence over precision, abbreviation and
    /// the thousands separator.
    #[must_use]
    pub fn with_numeric_formatter<F>(mut self, formatter: F) -> Self
    where
        F: Fn(f64) -> String + Send + Sync + 'static,
    {
        self.numeric = Some(Arc::new(formatter));
        self
    }

    /// Replaces the categorical/text rendering entirely. `prefix`/`suffix` still apply.
    #[must_use]
    pub fn with_text_formatter<F>(mut self, formatter: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.text = Some(Arc::new(formatter));
        self
    }

    /// Whether the format needs the number to be rebuilt from its value.
    ///
    /// When this is `false`, the label produced by the scale is kept and only
    /// decorated with `prefix`/`suffix`. It becomes `true` as soon as an option
    /// needs the numeric value itself: precision, a thousands separator, an
    /// abbreviation, or a multiplier other than `1`. `trim_zeros` alone does
    /// not, because it only has an effect once the number is rebuilt.
    fn has_numeric_options(&self) -> bool {
        self.precision.is_some()
            || self.thousands_separator.is_some()
            || self.abbreviation.is_some()
            || (self.multiplier - 1.0).abs() > f64::EPSILON
    }

    /// Formats a numeric tick. `base` is the label the scale produced, used as the
    /// body when no numeric option asks for a rewrite. See [`LabelFormat`] for the
    /// full option precedence.
    pub(crate) fn apply_numeric(&self, value: f64, base: &str) -> String {
        let body = if let Some(formatter) = &self.numeric {
            // `with_multiplier` composes with the closure instead of being ignored.
            formatter(value * self.multiplier)
        } else if self.has_numeric_options() {
            self.render_number(value)
        } else {
            base.to_string()
        };
        format!("{}{}{}", self.prefix, body, self.suffix)
    }

    /// Formats a categorical/date label.
    pub(crate) fn apply_text(&self, label: &str) -> String {
        let body = if let Some(formatter) = &self.text {
            formatter(label)
        } else {
            label.to_string()
        };
        format!("{}{}{}", self.prefix, body, self.suffix)
    }

    fn render_number(&self, value: f64) -> String {
        let scaled = value * self.multiplier;
        let (display, unit) = match self.abbreviation {
            Some(Abbreviation::Compact) => split_compact(scaled),
            None => (scaled, ""),
        };
        let mut out = self.format_decimal(display);
        out.push_str(unit);
        out
    }

    fn format_decimal(&self, value: f64) -> String {
        let raw = match self.precision {
            Some(precision) => format!("{value:.precision$}"),
            None => format!("{value}"),
        };
        let rendered = if self.trim_zeros {
            trim_decimal_zeros(&raw)
        } else {
            raw
        };
        match self.thousands_separator {
            Some(separator) => insert_thousands(&rendered, separator),
            None => rendered,
        }
    }
}

/// Splits a value into `(mantissa, suffix)` using compact notation.
fn split_compact(value: f64) -> (f64, &'static str) {
    let abs = value.abs();
    if abs == 0.0 {
        return (value, "");
    }

    const LARGE: [(f64, &str); 4] = [(1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "k")];
    const SMALL: [(f64, &str); 4] = [(1e-3, "m"), (1e-6, "µ"), (1e-9, "n"), (1e-12, "p")];

    if abs >= 1e3 {
        for (factor, unit) in LARGE {
            if abs >= factor {
                return (value / factor, unit);
            }
        }
    } else if abs < 1.0 {
        for (factor, unit) in SMALL {
            if abs >= factor {
                return (value / factor, unit);
            }
        }
    }
    (value, "")
}

/// Removes trailing zeros from the fractional part of a plain decimal string.
fn trim_decimal_zeros(text: &str) -> String {
    if !text.contains('.') {
        return text.to_string();
    }
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Inserts a separator every three digits of the integer part.
fn insert_thousands(text: &str, separator: char) -> String {
    let (sign, rest) = match text.strip_prefix('-') {
        Some(stripped) => ("-", stripped),
        None => ("", text),
    };
    let (integer, fraction) = match rest.find('.') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };

    let mut grouped = String::with_capacity(integer.len() + integer.len() / 3 + 1);
    for (index, character) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index).is_multiple_of(3) {
            grouped.push(separator);
        }
        grouped.push(character);
    }

    format!("{sign}{grouped}{fraction}")
}

/// A [`ScaleTrait`] wrapper that applies a [`LabelFormat`] to every tick the
/// wrapped scale produces.
///
/// It passes every calculation through to the wrapped scale and changes only
/// [`Tick::label`]. All ticks come from `suggest_ticks`, `create_explicit_ticks`
/// or `sample_n`, so changing those three methods covers every axis, grid line,
/// legend and colour bar.
///
/// The label text is written here, so the layout engine that measures labels and
/// the renderers that draw them always read the same strings.
#[derive(Debug, Clone)]
pub struct FormattedScale {
    inner: Arc<dyn ScaleTrait>,
    format: LabelFormat,
}

impl FormattedScale {
    /// Wraps `inner`, formatting every label it produces.
    pub(crate) fn new(inner: Arc<dyn ScaleTrait>, format: LabelFormat) -> Self {
        Self { inner, format }
    }

    /// Rewrites the label of every tick in place.
    ///
    /// A discrete scale carries category names, so it is formatted as text;
    /// every other scale carries a numeric `value`, which is formatted as a
    /// number (the base label is still passed through when there is nothing to
    /// rewrite).
    fn apply(&self, ticks: &mut [Tick]) {
        let discrete = matches!(self.inner.scale_type(), Scale::Discrete);
        for tick in ticks.iter_mut() {
            tick.label = if discrete {
                self.format.apply_text(&tick.label)
            } else {
                self.format.apply_numeric(tick.value, &tick.label)
            };
        }
    }
}

impl ScaleTrait for FormattedScale {
    fn scale_type(&self) -> Scale {
        self.inner.scale_type()
    }

    fn normalize(&self, value: f64) -> f64 {
        self.inner.normalize(value)
    }

    fn normalize_string(&self, value: &str) -> f64 {
        self.inner.normalize_string(value)
    }

    fn domain(&self) -> (f64, f64) {
        self.inner.domain()
    }

    fn logical_max(&self) -> f64 {
        self.inner.logical_max()
    }

    fn mapper(&self) -> Option<&VisualMapper> {
        self.inner.mapper()
    }

    fn suggest_ticks(&self, count: usize) -> Vec<Tick> {
        let mut ticks = self.inner.suggest_ticks(count);
        self.apply(&mut ticks);
        ticks
    }

    fn create_explicit_ticks(&self, explicit: &[ExplicitTick]) -> Vec<Tick> {
        let mut ticks = self.inner.create_explicit_ticks(explicit);
        self.apply(&mut ticks);
        ticks
    }

    fn get_domain_enum(&self) -> ScaleDomain {
        self.inner.get_domain_enum()
    }

    fn sample_n(&self, n: usize) -> Vec<Tick> {
        let mut ticks = self.inner.sample_n(n);
        self.apply(&mut ticks);
        ticks
    }

    fn label_formatter(&self) -> Option<&LabelFormat> {
        Some(&self.format)
    }
}

/// Wraps `scale` in a [`FormattedScale`] when a format is present, and returns it
/// unchanged otherwise, so charts without a custom format pay no extra
/// indirection.
pub(crate) fn format_scale(
    scale: Arc<dyn ScaleTrait>,
    format: Option<&LabelFormat>,
) -> Arc<dyn ScaleTrait> {
    match format {
        Some(format) => Arc::new(FormattedScale::new(scale, format.clone())),
        None => scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_notation_uses_billions() {
        let format = LabelFormat::new()
            .with_prefix("$")
            .with_compact_notation()
            .with_precision(0);
        assert_eq!(format.apply_numeric(20_000_000_000.0, "x"), "$20B");
        assert_eq!(format.apply_numeric(0.0, "x"), "$0");
        assert_eq!(format.apply_numeric(1_500.0, "x"), "$2k");
    }

    #[test]
    fn prefix_and_suffix_keep_the_default_number_format() {
        let format = LabelFormat::new().with_prefix("~").with_suffix(" kg");
        // No numeric option, so the scale-produced body is preserved.
        assert_eq!(format.apply_numeric(1.0, "1.0000E7"), "~1.0000E7 kg");
    }

    #[test]
    fn text_formatter_applies_to_categories() {
        let format = LabelFormat::new().with_text_formatter(|label| label.to_uppercase());
        assert_eq!(format.apply_text("north"), "NORTH");
    }

    #[test]
    fn thousands_separator_groups_the_integer_part() {
        let format = LabelFormat::new().with_thousands_separator(',');
        assert_eq!(format.apply_numeric(1_234_567.0, "x"), "1,234,567");
        assert_eq!(format.apply_numeric(-12_345.5, "x"), "-12,345.5");
    }

    #[test]
    fn custom_numeric_formatter_still_allows_decoration() {
        let format = LabelFormat::new()
            .with_suffix("%")
            .with_numeric_formatter(|value| format!("{:.1}", value * 100.0));
        assert_eq!(format.apply_numeric(0.5, "x"), "50.0%");
    }

    #[test]
    fn multiplier_composes_with_a_custom_formatter() {
        let format = LabelFormat::new()
            .with_multiplier(1e-9)
            .with_numeric_formatter(|seconds| format!("{seconds:.1}s"));
        assert_eq!(format.apply_numeric(2_500_000_000.0, "x"), "2.5s");
    }

    #[test]
    fn trim_zeros_can_be_disabled() {
        let format = LabelFormat::new().with_precision(2).with_trim_zeros(false);
        assert_eq!(format.apply_numeric(20.0, "x"), "20.00");
    }
}
