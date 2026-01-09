use super::{ScaleTrait, Tick};
use std::collections::HashMap;

/// A scale for categorical data that maps discrete string values to spatial slots.
/// 
/// In a `DiscreteScale`, the range is divided into equal slots (bands). 
/// Data points are typically centered within these slots, which is ideal 
/// for bar charts or categorical scatter plots.
pub struct DiscreteScale {
    /// The unique categorical labels in the order they should appear.
    domain: Vec<String>,
    /// A lookup map to provide O(1) performance when finding a category's index.
    index_map: HashMap<String, usize>,
    /// The output visual boundaries: (start_pixel, end_pixel).
    range: (f64, f64),
}

impl DiscreteScale {
    /// Creates a new `DiscreteScale` from a list of categories and a pixel range.
    /// 
    /// Internally builds an `index_map` for efficient mapping.
    pub fn new(domain: Vec<String>, range: (f64, f64)) -> Self {
        let mut index_map = HashMap::with_capacity(domain.len());
        for (i, val) in domain.iter().enumerate() {
            index_map.insert(val.clone(), i);
        }

        Self {
            domain,
            index_map,
            range,
        }
    }

    /// Maps a categorical string value directly to its calculated pixel coordinate.
    /// 
    /// If the value is not found in the domain, it defaults to the start of the range.
    pub fn map_string(&self, value: &str) -> f64 {
        match self.index_map.get(value) {
            Some(&index) => self.map(index as f64),
            None => self.range.0,
        }
    }

    /// Returns the total number of categories in the domain.
    pub fn count(&self) -> usize {
        self.domain.len()
    }
}

impl ScaleTrait for DiscreteScale {
    /// Maps a numeric index (as f64) to the center of its corresponding categorical slot.
    /// 
    /// The total range is divided by the number of categories `N`. 
    /// Each category occupies a slot of width `step`. This function returns 
    /// the pixel position at `r_min + (index * step) + (step / 2)`.
    fn map(&self, value: f64) -> f64 {
        let n = self.domain.len();
        if n == 0 { return self.range.0; }
        
        let (r_min, r_max) = self.range;
        
        // Single category is placed exactly in the middle of the available range.
        if n == 1 {
            return r_min + (r_max - r_min) / 2.0;
        }

        let step = (r_max - r_min) / (n as f64);
        
        // Position the point at the center of the band/slot.
        r_min + (value * step) + (step / 2.0)
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

    /// Returns the pixel boundaries (start, end).
    fn range(&self) -> (f64, f64) {
        self.range
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