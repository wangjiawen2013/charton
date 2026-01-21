use super::{ScaleTrait, ScaleDomain, Tick};
use std::collections::HashMap;

/// A scale for categorical data that maps discrete values to normalized slots.
/// 
/// In `Charton`, a `DiscreteScale` divides the normalized [0, 1] range into 
/// equal bands. Data points are centered within these bands, which is 
/// standard for bar charts or categorical dot plots.
/// 
/// Note: To allow for visual padding, this scale supports an expanded domain
/// where the coordinate range covers slightly more than the [0, N-1] index space.
#[derive(Debug, Clone)]
pub struct DiscreteScale {
    /// The unique categorical labels in the order they should appear.
    domain: Vec<String>,
    /// A lookup map to provide O(1) performance when finding a category's index.
    index_map: HashMap<String, usize>,
    /// The expanded index boundaries: (min_idx, max_idx).
    /// Typically (-0.6, N - 1 + 0.6) to provide space for bars/points.
    expanded_range: (f32, f32),
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
            let range = (n - 1) as f32;
            
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
    fn normalize(&self, value: f32) -> f32 {
        let (min, max) = self.expanded_range;
        let range = max - min;
        
        if range.abs() < f32::EPSILON { 
            return 0.5; 
        }
        
        // Map the index 'value' into the [min, max] expanded space.
        (value - min) / range
    }

    /// Implementation for DiscreteScale
    /// This uses the internal index_map to find the position of the string.
    fn normalize_string(&self, value: &str) -> f32 {
        match self.index_map.get(value) {
            Some(&index) => self.normalize(index as f32),
            None => 0.0, // Or perhaps a value that maps to "NA" color
        }
    }

    /// Returns the expanded domain boundaries (min, max) in index space.
    fn domain(&self) -> (f32, f32) {
        self.expanded_range
    }

    /// Returns the maximum logical index of the domain (N - 1).
    /// Used by VisualMapper to index into color palettes or shape lists.
    fn logical_max(&self) -> f32 {
        let n = self.domain.len();
        if n == 0 {
            0.0
        } else {
            (n - 1) as f32
        }
    }

    /// Returns a list of ticks where each category in the domain is a tick.
    /// 
    /// For discrete scales, the `count` argument is ignored because 
    /// all categories are typically significant enough to be labeled.
    fn ticks(&self, _count: usize) -> Vec<Tick> {
        self.domain.iter().enumerate().map(|(i, label)| {
            Tick {
                value: i as f32,
                label: label.clone(),
            }
        }).collect()
    }

    /// Implementation for DiscreteScale.
    /// Returns the original list of categories as a ScaleDomain::Categorical.
    fn get_domain_enum(&self) -> ScaleDomain {
        ScaleDomain::Categorical(self.domain.clone())
    }

    /// Provides a representative sample of categories for discrete guides.
    /// 
    /// For discrete scales, 'sampling' means picking a subset of labels. 
    /// If the domain size is smaller than N, it returns all categories.
    /// If larger, it picks N evenly spaced categories to represent the set.
    fn sample_n(&self, n: usize) -> Vec<Tick> {
        let len = self.domain.len();
        
        // Handle empty or small domains: return everything we have.
        if len <= n || n == 0 {
            return self.ticks(n);
        }

        // Otherwise, pick N indices that are spread as evenly as possible 
        // across the discrete set.
        (0..n).map(|i| {
            // Calculate the index using a floating step: 
            // e.g., if len=10, n=3, we might pick indices [0, 4, 9]
            let idx = if i == n - 1 {
                len - 1
            } else {
                ((i as f32 * (len - 1) as f32) / (n - 1) as f32).round() as usize
            };

            Tick {
                value: idx as f32,
                label: self.domain[idx].clone(),
            }
        }).collect()
    }
}