use super::{Scale, ScaleDomain, ScaleTrait, Tick, ExplicitTick, mapper::VisualMapper};
use std::collections::HashMap;

/// A scale for categorical data that maps discrete values to normalized slots.
///
/// In `Charton`, a `DiscreteScale` divides the normalized [0, 1] range into
/// equal bands. Data points are centered within these bands, which is
/// standard for bar charts, box plots, or categorical dot plots.
///
/// For visual mapping (Color, Shape), this scale uses the integer index of the
/// category to look up values in a palette or list via the `VisualMapper`.
#[derive(Debug, Clone)]
pub struct DiscreteScale {
    /// The unique categorical labels in the order they should appear.
    domain: Vec<String>,

    /// A lookup map to provide O(1) performance when finding a category's index.
    index_map: HashMap<String, usize>,

    /// The expanded index boundaries: (min_idx, max_idx).
    /// Typically (-0.6, N - 1 + 0.6) to provide breathing room for visual marks
    /// like bars so they don't touch the axis edges.
    expanded_range: (f64, f64),

    /// The optional visual mapper used to convert discrete indices
    /// into aesthetics like specific colors or shapes.
    mapper: Option<VisualMapper>,
}

impl DiscreteScale {
    /// Creates a new `DiscreteScale` from a list of categories and expansion settings.
    ///
    /// # Arguments
    /// * `domain` - A vector of unique category strings.
    /// * `expand` - The expansion constants (usually add: 0.6 for discrete scales).
    /// * `mapper` - Optional visual mapping logic for categorical aesthetics.
    pub fn new(
        domain: Vec<String>,
        expansion: crate::scale::Expansion,
        mapper: Option<VisualMapper>,
    ) -> Self {
        let mut index_map = HashMap::with_capacity(domain.len());
        for (i, val) in domain.iter().enumerate() {
            index_map.insert(val.clone(), i);
        }

        let n = domain.len();
        let expanded_range = if n == 0 {
            (0.0, 0.0)
        } else {
            // Calculate padding in index space.
            let range = (n - 1) as f64;

            let lower_padding = range * expansion.mult.0 + expansion.add.0;
            let upper_padding = range * expansion.mult.1 + expansion.add.1;

            (0.0 - lower_padding, range + upper_padding)
        };

        Self {
            domain,
            index_map,
            expanded_range,
            mapper,
        }
    }

    /// Returns the total number of categories in the domain.
    pub fn count(&self) -> usize {
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
    /// Returns 0.0 if the category is not found in the domain.
    fn normalize_string(&self, value: &str) -> f64 {
        match self.index_map.get(value) {
            Some(&index) => self.normalize(index as f64),
            None => 0.0,
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

    /// Returns the raw category list as a Categorical domain variant.
    fn get_domain_enum(&self) -> ScaleDomain {
        ScaleDomain::Discrete(self.domain.clone())
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
