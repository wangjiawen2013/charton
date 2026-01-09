use super::{ScaleTrait, Tick};
use std::collections::HashMap;

/// A scale for categorical data that maps discrete values to normalized slots.
/// 
/// In `Charton`, a `DiscreteScale` divides the normalized [0, 1] range into 
/// equal bands. Data points are centered within these bands, which is 
/// standard for bar charts or categorical dot plots.
pub struct DiscreteScale {
    /// The unique categorical labels in the order they should appear.
    domain: Vec<String>,
    /// A lookup map to provide O(1) performance when finding a category's index.
    index_map: HashMap<String, usize>,
}

impl DiscreteScale {
    /// Creates a new `DiscreteScale` from a list of categories.
    /// 
    /// Internally builds an `index_map` for efficient lookup during the 
    /// normalization process.
    pub fn new(domain: Vec<String>) -> Self {
        let mut index_map = HashMap::with_capacity(domain.len());
        for (i, val) in domain.iter().enumerate() {
            index_map.insert(val.clone(), i);
        }

        Self {
            domain,
            index_map,
        }
    }

    /// Maps a categorical string value to its normalized [0, 1] position.
    /// 
    /// If the value is not found in the domain, it defaults to 0.0.
    pub fn normalize_string(&self, value: &str) -> f64 {
        match self.index_map.get(value) {
            Some(&index) => self.normalize(index as f64),
            None => 0.0,
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
    /// In ggplot2, discrete values are mapped to the center of their respective
    /// bands. If there are 3 categories, they divide the space into 3 bands:
    /// Band 0: [0.0, 0.33], Band 1: [0.33, 0.66], Band 2: [0.66, 1.0]
    /// The centers (and thus the return values) would be 0.166, 0.5, and 0.833.
    fn normalize(&self, value: f64) -> f64 {
        let n = self.domain.len() as f64;
        
        // Handling empty domain:
        // ggplot2 typically handles this at a higher level, but for a robust Scale implementation,
        // returning 0.5 represents a neutral position (the center) when no extent exists.
        if n < 1.0 { 
            return 0.5; 
        }
        
        // Formula: (index + 0.5) / n
        // This ensures the point is perfectly centered in its categorical band.
        (value + 0.5) / n
    }

    /// Returns the domain as a range of indices: `(0.0, N - 1)`.
    fn domain(&self) -> (f64, f64) {
        let n = self.domain.len();
        if n == 0 {
            (0.0, 0.0)
        } else {
            (0.0, (n - 1) as f64)
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