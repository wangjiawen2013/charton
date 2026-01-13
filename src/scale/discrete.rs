use super::{ScaleTrait, Tick};
use std::collections::HashMap;

/// A scale for categorical data that maps discrete values to normalized slots.
/// 
/// In `Charton`, a `DiscreteScale` divides the normalized [0, 1] range into 
/// equal bands. Data points are centered within these bands, which is 
/// standard for bar charts or categorical dot plots.
/// 
/// Note: To allow for visual padding, this scale supports an expanded domain
/// where the coordinate range covers slightly more than the [0, N-1] index space.
pub struct DiscreteScale {
    /// The unique categorical labels in the order they should appear.
    domain: Vec<String>,
    /// A lookup map to provide O(1) performance when finding a category's index.
    index_map: HashMap<String, usize>,
    /// The expanded index boundaries: (min_idx, max_idx).
    /// Typically (-0.6, N - 1 + 0.6) to provide space for bars/points.
    expanded_range: (f64, f64),
}

impl DiscreteScale {
    /// Creates a new `DiscreteScale` from a list of categories.
    /// 
    /// Internally builds an `index_map` for efficient lookup and sets the 
    /// coordinate boundaries.
    pub fn new(domain: Vec<String>, expand: crate::scale::Expansion) -> Self {
        let mut index_map = HashMap::with_capacity(domain.len());
        for (i, val) in domain.iter().enumerate() {
            index_map.insert(val.clone(), i);
        }

        let n = domain.len();
        let expanded_range = if n == 0 {
            (0.0, 0.0)
        } else {
            // Apply discrete expansion: 
            // ggplot2 default mult is usually 0, and add is 0.6.
            // With asymmetric expansion, we apply (mult.0, add.0) to the lower bound
            // and (mult.1, add.1) to the upper bound.
            let range = (n - 1) as f64;
            
            let lower_padding = range * expand.mult.0 + expand.add.0;
            let upper_padding = range * expand.mult.1 + expand.add.1;

            (0.0 - lower_padding, range + upper_padding)
        };

        Self {
            domain,
            index_map,
            expanded_range,
        }
    }

    /// Returns the total number of categories in the domain.
    pub fn count(&self) -> usize {
        self.domain.len()
    }
}

impl ScaleTrait for DiscreteScale {
    /// Transforms a categorical index into a normalized [0, 1] ratio.
    /// 
    /// In a discrete scale with expansion, the "0.0" and "1.0" on the screen 
    /// correspond to the expanded_range limits (e.g., -0.6 and N-0.4).
    /// This ensures categories are centered and have padding.
    fn normalize(&self, value: f64) -> f64 {
        let (min, max) = self.expanded_range;
        let range = max - min;
        
        if range.abs() < f64::EPSILON { 
            return 0.5; 
        }
        
        // Map the index 'value' into the [min, max] expanded space.
        (value - min) / range
    }

    /// Implementation for DiscreteScale
    /// This uses the internal index_map to find the position of the string.
    fn normalize_string(&self, value: &str) -> f64 {
        match self.index_map.get(value) {
            Some(&index) => self.normalize(index as f64),
            None => 0.0, // Or perhaps a value that maps to "NA" color
        }
    }

    /// Returns the expanded domain boundaries (min, max) in index space.
    fn domain(&self) -> (f64, f64) {
        self.expanded_range
    }

    /// Returns the maximum logical index of the domain (N - 1).
    /// Used by VisualMapper to index into color palettes or shape lists.
    fn logical_max(&self) -> f64 {
        let n = self.domain.len();
        if n == 0 {
            0.0
        } else {
            (n - 1) as f64
        }
    }

    /// Returns a list of ticks where each category in the domain is a tick.
    /// 
    /// For discrete scales, the `count` argument is ignored because 
    /// all categories are typically significant enough to be labeled.
    fn ticks(&self, _count: usize) -> Vec<Tick> {
        self.domain.iter().enumerate().map(|(i, label)| {
            Tick {
                value: i as f64,
                label: label.clone(),
            }
        }).collect()
    }
}