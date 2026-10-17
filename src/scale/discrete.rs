use super::{ExplicitTick, Scale, ScaleDomain, ScaleTrait, Tick, mapper::VisualMapper};
use ahash::AHashMap;

/// A scale for categorical data that maps discrete values to normalized slots.
///
/// In `Charton`, a `DiscreteScale` divides the normalized [0, 1] range into
/// equal bands. Data points are centered within these bands, which is
/// standard for bar charts, box plots, or categorical dot plots.
///
/// For visual mapping (Color, Shape), this scale uses the integer index of the
/// category to look up values in a palette or list via the `VisualMapper`.
///
/// # Numeric category positions
///
/// A plain categorical axis places every observation exactly on a category
/// index (`0, 1, 2, ...`). Some marks, however, use the category index as a
/// *centre* and place geometry around it: a violin spans `centre +/- width`, a
/// dodged lane sits at `centre + offset`, and a ridgeline with `overlap` grows
/// several categories past its baseline. When the scale is trained from a
/// numeric position column (see [`X::with_category_labels`]), those positions
/// are carried as a `position_extent`. The expanded range then covers both the
/// ticked category indices and the positions the marks actually occupy, so a
/// mark can never be silently clipped by the panel boundary.
///
/// [`X::with_category_labels`]: crate::encode::x::X::with_category_labels
#[derive(Debug, Clone)]
pub struct DiscreteScale {
    /// The unique categorical labels in the order they should appear.
    domain: Vec<String>,

    /// A lookup map to provide O(1) performance when finding a category's index.
    index_map: AHashMap<String, usize>,

    /// The range in index space that the axis displays.
    ///
    /// For a plain categorical axis this is typically `(-0.4, N - 1 + 0.4)` or
    /// `(-0.5, N - 1 + 0.5)`, depending on the mark. When the data occupies
    /// numeric positions outside that range (an overlapping ridgeline, a dodged
    /// lane at the edge), it grows to include them plus the axis padding.
    expanded_range: (f64, f64),

    /// The numeric position extent the scale was trained from, when the
    /// encoding used `with_category_labels`. Kept so the trained domain can be
    /// reported back faithfully.
    position_extent: Option<(f64, f64)>,

    /// The optional visual mapper used to convert discrete indices
    /// into aesthetics like specific colors or shapes.
    mapper: Option<VisualMapper>,
}

impl DiscreteScale {
    /// Creates a new `DiscreteScale` from a list of categories and expansion settings.
    ///
    /// # Arguments
    /// * `domain` - A vector of unique category strings.
    /// * `expansion` - The expansion constants (usually `add: 0.4` or `0.5` for
    ///   discrete scales).
    /// * `mapper` - Optional visual mapping logic for categorical aesthetics.
    pub fn new(
        domain: Vec<String>,
        expansion: crate::scale::Expansion,
        mapper: Option<VisualMapper>,
    ) -> Self {
        Self::with_positions(domain, expansion, None, mapper)
    }

    /// Creates a discrete scale that also knows the numeric positions its data
    /// occupies.
    ///
    /// The axis always shows the category indices (`0..len-1`) as ticks. When
    /// `position_extent` reaches past the padded index range, the expanded
    /// range grows to include it, with the same per-side padding the axis
    /// requested. Positions that already fit leave the range untouched, so a
    /// normal violin or bar chart keeps its existing geometry.
    pub fn with_positions(
        domain: Vec<String>,
        expansion: crate::scale::Expansion,
        position_extent: Option<(f64, f64)>,
        mapper: Option<VisualMapper>,
    ) -> Self {
        let mut index_map = AHashMap::with_capacity(domain.len());
        for (i, val) in domain.iter().enumerate() {
            index_map.insert(val.clone(), i);
        }

        // A non-finite or inverted extent carries no information; treat it as
        // if the scale had been trained from the category indices alone.
        let position_extent =
            position_extent.filter(|(min, max)| min.is_finite() && max.is_finite() && min <= max);

        let expanded_range = Self::resolve_range(&domain, expansion, position_extent);

        Self {
            domain,
            index_map,
            expanded_range,
            position_extent,
            mapper,
        }
    }

    /// Resolves the displayed range from the category indices, the requested
    /// expansion, and the actual data positions.
    ///
    /// The category indices are the axis proper: they define where the ticks
    /// sit. The expansion is the room the axis asks for around those indices
    /// (a bar's half width, a violin's half width, ...). A mark that reaches
    /// outside that room is data the axis has to show, not an error, so the
    /// range grows to include it and keeps the same padding on that side.
    fn resolve_range(
        domain: &[String],
        expansion: crate::scale::Expansion,
        position_extent: Option<(f64, f64)>,
    ) -> (f64, f64) {
        let n = domain.len();
        if n == 0 {
            return (0.0, 0.0);
        }

        // Padding is measured in category steps. It is deliberately derived
        // from the category span, not from the data extent: the gap around a
        // mark should stay the same however far that mark overhangs.
        let category_span = (n - 1) as f64;
        let lower_padding = category_span * expansion.mult.0 + expansion.add.0;
        let upper_padding = category_span * expansion.mult.1 + expansion.add.1;

        let mut lower = -lower_padding;
        let mut upper = category_span + upper_padding;

        if let Some((data_min, data_max)) = position_extent {
            // Only grow. Geometry that lands exactly on the padded edge (a
            // violin's half width is designed to) must not be widened by a
            // stray floating-point bit.
            const EDGE_EPSILON: f64 = 1e-9;
            if data_min < lower - EDGE_EPSILON {
                lower = data_min - lower_padding;
            }
            if data_max > upper + EDGE_EPSILON {
                upper = data_max + upper_padding;
            }
        }

        (lower, upper)
    }

    /// Returns the total number of categories in the domain.
    pub const fn count(&self) -> usize {
        self.domain.len()
    }
}

impl ScaleTrait for DiscreteScale {
    fn scale_type(&self) -> Scale {
        Scale::Discrete
    }

    /// Transforms a categorical index into a normalized [0, 1] ratio.
    ///
    /// Because of the `expanded_range`, an index of 0 will not map to 0.0 on screen,
    /// but rather to a slightly offset value, ensuring the first category
    /// has visual padding from the axis.
    fn normalize(&self, value: f64) -> f64 {
        let (min, max) = self.expanded_range;
        let range = max - min;

        if range.abs() < 1e-9 {
            return 0.5;
        }

        // Map the index 'value' into the [min, max] expanded coordinate space.
        (value - min) / range
    }

    /// Maps a string label to its normalized position.
    /// Returns f64::NAN if the category is not found, allowing the
    /// renderer to skip the mark rather than placing it at the origin.
    fn normalize_string(&self, value: &str) -> f64 {
        match self.index_map.get(value) {
            Some(&index) => self.normalize(index as f64),
            None => f64::NAN,
        }
    }

    /// Returns the expanded boundaries in index space (e.g., -0.6 to 4.6 for 5 categories).
    fn domain(&self) -> (f64, f64) {
        self.expanded_range
    }

    /// Returns the maximum index (N - 1).
    ///
    /// This is crucial for `VisualMapper` when using indexed palettes (e.g., Color1, Color2...).
    /// It tells the mapper the total number of discrete steps available.
    fn logical_max(&self) -> f64 {
        let n = self.domain.len();
        if n == 0 { 0.0 } else { (n - 1) as f64 }
    }

    /// Returns the associated `VisualMapper` for this discrete scale.
    fn mapper(&self) -> Option<&VisualMapper> {
        self.mapper.as_ref()
    }

    /// Generates ticks for every category in the domain.
    ///
    /// For discrete scales, we ignore the requested `count` because every
    /// category is typically an essential label on the axis.
    fn suggest_ticks(&self, _count: usize) -> Vec<Tick> {
        self.domain
            .iter()
            .enumerate()
            .map(|(i, label)| Tick {
                value: i as f64,
                label: label.clone(),
            })
            .collect()
    }

    /// Transforms user-defined discrete ticks into renderable Tick objects.
    ///
    /// For discrete scales, we map the provided string labels to their
    /// corresponding integer indices defined in the domain.
    fn create_explicit_ticks(&self, explicit: &[ExplicitTick]) -> Vec<Tick> {
        let mut type_mismatch = 0;
        let mut not_in_domain = 0;

        let ticks: Vec<Tick> = explicit
            .iter()
            .filter_map(|tick| {
                match tick {
                    ExplicitTick::Discrete(label) => {
                        // Find the index of this label in our domain categories
                        if let Some(index) = self.domain.iter().position(|d| d == label) {
                            Some(Tick {
                                value: index as f64,
                                label: label.clone(),
                            })
                        } else {
                            not_in_domain += 1;
                            None
                        }
                    }
                    // Handle cases where user passed Continuous or Temporal to a Discrete scale
                    _ => {
                        type_mismatch += 1;
                        None
                    }
                }
            })
            .collect();

        // Bulk warning for debugging
        if type_mismatch > 0 || not_in_domain > 0 {
            eprintln!(
                "Warning [DiscreteScale]: Filtered {} ticks ({} type mismatch, {} not found in domain).",
                type_mismatch + not_in_domain,
                type_mismatch,
                not_in_domain
            );
        }

        ticks
    }

    /// Returns the raw category list as a categorical domain variant.
    ///
    /// The numeric position extent is included when the scale was trained from
    /// one, so consumers can see the full trained domain rather than only the
    /// tick labels.
    fn get_domain_enum(&self) -> ScaleDomain {
        match self.position_extent {
            Some(extent) => ScaleDomain::DiscretePositions {
                labels: self.domain.clone(),
                extent,
            },
            None => ScaleDomain::Discrete(self.domain.clone()),
        }
    }

    /// Provides a sample of categories when the total count is too large for a legend.
    ///
    /// If the domain is small, it returns all categories. If it exceeds `n`,
    /// it selects `n` evenly distributed categories from the ordered set.
    fn sample_n(&self, n: usize) -> Vec<Tick> {
        let len = self.domain.len();

        if len <= n || n == 0 {
            return self.suggest_ticks(n);
        }

        (0..n)
            .map(|i| {
                let idx = if i == n - 1 {
                    len - 1
                } else {
                    ((i as f64 * (len - 1) as f64) / (n - 1) as f64).round() as usize
                };

                Tick {
                    value: idx as f64,
                    label: self.domain[idx].clone(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::Expansion;

    fn scale(n: usize, extent: Option<(f64, f64)>, add: f64) -> DiscreteScale {
        let labels = (0..n).map(|i| format!("c{i}")).collect();
        DiscreteScale::with_positions(
            labels,
            Expansion {
                mult: (0.0, 0.0),
                add: (add, add),
            },
            extent,
            None,
        )
    }

    #[test]
    fn positions_do_not_change_a_domain_that_already_fits() {
        assert_eq!(scale(3, None, 0.4).domain(), (-0.4, 2.4));
        assert_eq!(scale(3, Some((0.0, 2.2)), 0.4).domain(), (-0.4, 2.4));
        // A half width designed to land exactly on the padded edge must not be
        // widened by floating-point noise.
        assert_eq!(scale(3, Some((0.0, 2.4)), 0.4).domain(), (-0.4, 2.4));
    }

    #[test]
    fn positions_grow_the_domain_by_the_axis_padding() {
        // A ridgeline: the top ridge reaches 1.25 category steps past the last
        // category index (overlap of 2.5 on a one-step-wide band).
        assert_eq!(scale(3, Some((0.0, 3.25)), 0.4).domain(), (-0.4, 3.65));
        // A left-leaning half band grows below the first category.
        assert_eq!(scale(3, Some((-1.25, 2.0)), 0.4).domain(), (-1.65, 2.4));
    }

    #[test]
    fn non_finite_positions_are_ignored() {
        assert_eq!(
            scale(3, Some((f64::NAN, f64::INFINITY)), 0.4).domain(),
            (-0.4, 2.4)
        );
    }

    #[test]
    fn an_empty_domain_is_still_safe() {
        assert_eq!(scale(0, Some((0.0, 1.0)), 0.4).domain(), (0.0, 0.0));
    }
}
