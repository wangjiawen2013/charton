//! Binning: equal-width edges over a column's extent.
//!
//! The only binning implementation in the engine. `hist` and `rect` bin their
//! input with it and the `bin` transform emits the same edges, so a pre-binned
//! chart and a mark-binned one land on identical boundaries.
//!
//! The rule: `count` equal-width bins over `[min, max]`; a value falls in
//! `floor((v - min) / width)`, clamped to the last bin. A constant column has
//! zero width, so every row sits on one position.
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;

/// The edges a column is binned into.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinEdges {
    start: f64,
    width: f64,
    count: usize,
}

impl BinEdges {
    /// Edges covering `column` in `count` equal-width bins.
    ///
    /// Returns `None` when the column has no valid values or `count` is zero:
    /// there is nothing to bin, and inventing a range would draw a chart of
    /// made-up numbers.
    #[must_use]
    pub fn new(column: &ColumnVector, count: usize) -> Option<Self> {
        if count == 0 {
            return None;
        }
        let (min, max) = extent(column)?;

        // With one bin there is no interior boundary, and the historical width
        // of 1.0 keeps a single-bin chart where it has always been. A constant
        // column yields a zero width, which puts every row on one position — the
        // honest rendering of such data.
        let width = if count > 1 {
            (max - min) / count as f64
        } else {
            1.0
        };

        Some(Self {
            start: min,
            width,
            count,
        })
    }

    /// The bin count a mark picks when the document does not say.
    ///
    /// A square-root rule over the distinct values, clamped to `5..=50` so the
    /// axis stays readable, and one bin per category for discrete data. This is
    /// the rule `hist` and `rect` already used; it lives here so the `bin`
    /// transform can produce what they would have produced.
    #[must_use]
    pub fn default_count(column: &ColumnVector) -> usize {
        let unique = column.n_unique();
        match column.semantic_type() {
            crate::core::data::SemanticType::Continuous
            | crate::core::data::SemanticType::Temporal => {
                if unique <= 1 {
                    1
                } else {
                    ((unique as f64).sqrt() as usize).clamp(5, 50)
                }
            }
            crate::core::data::SemanticType::Discrete => unique.max(1),
        }
    }

    /// Which bin a value belongs to.
    #[must_use]
    pub fn index_of(&self, value: f64) -> usize {
        if self.width <= 0.0 || !value.is_finite() {
            return 0;
        }
        let index = ((value - self.start) / self.width).floor();
        if index <= 0.0 {
            0
        } else {
            // `as usize` saturates, and the clamp keeps the maximum value in the
            // last bin instead of creating a phantom bin at the top.
            (index as usize).min(self.count - 1)
        }
    }

    /// Left edge of a bin.
    #[must_use]
    pub fn start_of(&self, index: usize) -> f64 {
        self.start + index as f64 * self.width
    }

    /// Middle of a bin — the position a mark draws at.
    #[must_use]
    pub fn center_of(&self, index: usize) -> f64 {
        self.start + (index as f64 + 0.5) * self.width
    }

    /// Right edge of a bin.
    #[must_use]
    pub fn end_of(&self, index: usize) -> f64 {
        self.start + (index as f64 + 1.0) * self.width
    }

    /// Number of bins.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }

    /// Bin width.
    #[must_use]
    pub const fn width(&self) -> f64 {
        self.width
    }
}

/// Smallest and largest valid value, or `None` when the column has none.
fn extent(column: &ColumnVector) -> Option<(f64, f64)> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut found = false;

    for row in 0..column.len() {
        if let Some(value) = column.get(row).to_f64() {
            min = min.min(value);
            max = max.max(value);
            found = true;
        }
    }

    found.then_some((min, max))
}

/// A `bin` transform: write the edges of each row's bin as two new columns.
#[derive(Debug, Clone, PartialEq)]
pub struct Bin {
    field: String,
    maxbins: Option<usize>,
    start_name: String,
    end_name: String,
}

impl Bin {
    /// Bin `field`, naming the two output columns.
    ///
    /// # Errors
    ///
    /// Fails when the two output names are equal.
    pub fn new(
        field: impl Into<String>,
        maxbins: Option<usize>,
        start_name: impl Into<String>,
        end_name: impl Into<String>,
    ) -> Result<Self, ChartonError> {
        let (start_name, end_name) = (start_name.into(), end_name.into());
        if start_name == end_name {
            return Err(ChartonError::Data(
                "bin cannot write the same column name twice".into(),
            ));
        }
        if maxbins == Some(0) {
            return Err(ChartonError::Data("bin needs at least one bin".into()));
        }
        Ok(Self {
            field: field.into(),
            maxbins,
            start_name,
            end_name,
        })
    }

    /// Default output names, as Vega-Lite writes them.
    #[must_use]
    pub fn default_names(field: &str) -> (String, String) {
        (format!("bin_{field}"), format!("bin_{field}_end"))
    }

    /// The two output column names.
    #[must_use]
    pub fn output_names(&self) -> (&str, &str) {
        (&self.start_name, &self.end_name)
    }

    /// The dataset with the two edge columns appended. Every input column and
    /// every row is kept: binning annotates, it does not summarise.
    ///
    /// A row whose value is missing gets two nulls — it is not dropped, because
    /// dropping would silently change the row count a later aggregate sees.
    ///
    /// # Errors
    ///
    /// Fails when the column does not exist, or an output name would overwrite
    /// an existing column.
    pub fn apply(&self, data: &Dataset) -> Result<Dataset, ChartonError> {
        let column = data.column(&self.field)?;

        for name in [&self.start_name, &self.end_name] {
            if data.get_column_names().contains(name) {
                return Err(ChartonError::Data(format!(
                    "bin would overwrite the existing column '{name}'; name the outputs with `as`"
                )));
            }
        }

        let count = self
            .maxbins
            .unwrap_or_else(|| BinEdges::default_count(column));

        let mut starts = Vec::with_capacity(data.height());
        let mut ends = Vec::with_capacity(data.height());

        match BinEdges::new(column, count) {
            Some(edges) => {
                for row in 0..data.height() {
                    match column.get(row).to_f64() {
                        Some(value) => {
                            let index = edges.index_of(value);
                            starts.push(Some(edges.start_of(index)));
                            ends.push(Some(edges.end_of(index)));
                        }
                        None => {
                            starts.push(None);
                            ends.push(None);
                        }
                    }
                }
            }
            // No valid values: the columns still exist, so the encoding resolves,
            // and every cell is null.
            None => {
                starts.resize(data.height(), None);
                ends.resize(data.height(), None);
            }
        }

        let mut binned = data.clone();
        binned.add_column(self.start_name.clone(), starts)?;
        binned.add_column(self.end_name.clone(), ends)?;
        Ok(binned)
    }
}

impl<T: Mark> Chart<T> {
    /// Adds the edges of each row's bin as two columns.
    ///
    /// # Errors
    ///
    /// Fails when the column does not exist, or an output name would overwrite
    /// an existing column.
    pub fn transform_bin(mut self, bin: &Bin) -> Result<Self, ChartonError> {
        self.data = bin.apply(&self.data)?;
        if let Some(source) = self.source_data.take() {
            self.source_data = Some(bin.apply(&source)?);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Bin, BinEdges};
    use crate::core::data::{ColumnVector, Dataset};

    fn values() -> ColumnVector {
        ColumnVector::from(vec![0.0_f64, 2.5, 5.0, 9.9, 10.0])
    }

    #[test]
    fn edges_cover_the_extent_with_equal_width() {
        let edges = BinEdges::new(&values(), 5).expect("edges");
        assert_eq!(edges.count(), 5);
        assert!((edges.start_of(0) - 0.0).abs() < 1e-9);
        assert!((edges.end_of(4) - 10.0).abs() < 1e-9);
        assert!((edges.width() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn the_maximum_lands_in_the_last_bin() {
        let edges = BinEdges::new(&values(), 5).expect("edges");
        assert_eq!(edges.index_of(10.0), 4);
        assert_eq!(edges.index_of(0.0), 0);
        assert_eq!(edges.index_of(2.5), 1);
    }

    #[test]
    fn a_constant_column_has_no_width_and_one_position() {
        let constant = ColumnVector::from(vec![7.0_f64, 7.0, 7.0]);
        let edges = BinEdges::new(&constant, 5).expect("edges");
        assert_eq!(edges.width(), 0.0);
        assert!((edges.center_of(edges.index_of(7.0)) - 7.0).abs() < 1e-9);
    }

    #[test]
    fn an_empty_or_null_column_has_no_edges() {
        let empty = ColumnVector::from(Vec::<f64>::new());
        assert!(BinEdges::new(&empty, 5).is_none());
        let nulls = ColumnVector::from(vec![None::<f64>, None]);
        assert!(BinEdges::new(&nulls, 5).is_none());
    }

    #[test]
    fn the_transform_appends_two_columns_and_keeps_rows() {
        let mut data = Dataset::new();
        data.add_column("v", vec![0.0_f64, 5.0, 10.0]).expect("v");

        let binned = Bin::new("v", Some(2), "lo", "hi")
            .expect("bin")
            .apply(&data)
            .expect("apply");

        assert_eq!(binned.height(), 3);
        assert_eq!(binned.column("lo").expect("lo").get(0).to_f64(), Some(0.0));
        assert_eq!(binned.column("hi").expect("hi").get(0).to_f64(), Some(5.0));
    }

    #[test]
    fn a_missing_value_yields_nulls_not_a_dropped_row() {
        let mut data = Dataset::new();
        data.add_column("v", vec![Some(0.0_f64), None]).expect("v");

        let binned = Bin::new("v", Some(2), "lo", "hi")
            .expect("bin")
            .apply(&data)
            .expect("apply");

        assert_eq!(binned.height(), 2);
        assert!(binned.is_null("lo", 1));
    }
}
