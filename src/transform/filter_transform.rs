//! Row filtering.
//!
//! A predicate is data: `Equal`, `Range` or `OneOf` over one column. A filter
//! that has to compute something is not representable here; that is what
//! `transform_calculate` is for, and it exists only in Rust.
//!
//! Filtering removes rows and keeps every column and value as it was.
use crate::chart::Chart;
use crate::core::data::{AnyValue, ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;

/// A value a predicate compares against.
///
/// Two kinds only, because these are the two kinds a data cell can hold that a
/// comparison makes sense for. A category is matched as text, a measure as a
/// number; nothing is coerced between them.
#[derive(Debug, Clone, PartialEq)]
pub enum PredicateValue {
    /// A number, compared exactly against the cell's numeric reading.
    Number(f64),
    /// Text, compared against the cell's string form.
    Text(String),
}

/// A row predicate.
///
/// The three forms are Vega-Lite's, and they are kept apart rather than folded
/// into one "condition" object so that each can be validated precisely.
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    /// Keep rows whose `field` equals `value`.
    Equal {
        /// Column to test.
        field: String,
        /// Value the cell must equal.
        value: PredicateValue,
    },
    /// Keep rows whose `field` falls inside `[min, max]`. Either bound may be
    /// open. Null cells never match.
    Range {
        /// Column to test.
        field: String,
        /// Inclusive lower bound, if any.
        min: Option<f64>,
        /// Inclusive upper bound, if any.
        max: Option<f64>,
    },
    /// Keep rows whose `field` equals any of `values`.
    OneOf {
        /// Column to test.
        field: String,
        /// Acceptable values.
        values: Vec<PredicateValue>,
    },
}

impl Predicate {
    /// The column the predicate reads.
    #[must_use]
    pub fn field(&self) -> &str {
        match self {
            Self::Equal { field, .. } | Self::Range { field, .. } | Self::OneOf { field, .. } => {
                field
            }
        }
    }

    /// The dataset reduced to the rows this predicate accepts.
    ///
    /// # Errors
    ///
    /// Fails when the column does not exist; everything else is a value test.
    pub fn keep(&self, data: &Dataset) -> Result<Dataset, ChartonError> {
        let column = data.column(self.field())?;
        let keep: Vec<usize> = (0..data.height())
            .filter(|&row| self.accepts(column, row))
            .collect();
        data.take_rows(&keep)
    }

    /// Whether one cell passes.
    fn accepts(&self, column: &ColumnVector, row: usize) -> bool {
        let cell = column.get(row);
        match self {
            Self::Equal { value, .. } => value.matches(&cell),
            // A missing value is not inside any range: `null` has no magnitude,
            // so claiming it is "within 0..10" would be a fabrication.
            Self::Range { min, max, .. } => cell.to_f64().is_some_and(|value| {
                min.is_none_or(|min| value >= min) && max.is_none_or(|max| value <= max)
            }),
            Self::OneOf { values, .. } => values.iter().any(|value| value.matches(&cell)),
        }
    }
}

impl PredicateValue {
    fn matches(&self, cell: &AnyValue<'_>) -> bool {
        match self {
            Self::Number(number) => cell.to_f64().is_some_and(|value| value == *number),
            Self::Text(text) => cell.to_string().is_some_and(|value| value == *text),
        }
    }
}

impl<T: Mark> Chart<T> {
    /// Keeps only the rows the predicate accepts.
    ///
    /// Runs on the dataset the chart currently holds, so the builder order
    /// decides the order of operations: call it before `mark_*()` to filter the
    /// input, or after a statistical transform to filter its output.
    ///
    /// The chart's pre-statistic copy of the rows is filtered as well, so a
    /// faceted chart re-runs its statistics on filtered data rather than on the
    /// original table.
    ///
    /// # Errors
    ///
    /// Fails when the predicate names a column that does not exist.
    pub fn transform_filter(mut self, predicate: &Predicate) -> Result<Self, ChartonError> {
        self.data = predicate.keep(&self.data)?;
        if let Some(source) = self.source_data.take() {
            self.source_data = Some(predicate.keep(&source)?);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Predicate, PredicateValue};
    use crate::core::data::Dataset;

    fn sample() -> Dataset {
        let mut data = Dataset::new();
        data.add_column("size", vec![1.0_f64, 2.0, 3.0, 4.0])
            .expect("size column");
        data.add_column(
            "region",
            vec![
                "north".to_owned(),
                "south".to_owned(),
                "north".to_owned(),
                "west".to_owned(),
            ],
        )
        .expect("region column");
        data
    }

    #[test]
    fn equal_keeps_matching_text_rows() {
        let data = sample();
        let predicate = Predicate::Equal {
            field: "region".to_owned(),
            value: PredicateValue::Text("north".to_owned()),
        };
        assert_eq!(predicate.keep(&data).expect("filter").height(), 2);
    }

    #[test]
    fn range_is_inclusive_and_drops_nulls() {
        let data = sample();
        let predicate = Predicate::Range {
            field: "size".to_owned(),
            min: Some(2.0),
            max: Some(3.0),
        };
        assert_eq!(predicate.keep(&data).expect("filter").height(), 2);
    }

    #[test]
    fn one_of_keeps_every_listed_value() {
        let data = sample();
        let predicate = Predicate::OneOf {
            field: "region".to_owned(),
            values: vec![
                PredicateValue::Text("south".to_owned()),
                PredicateValue::Text("west".to_owned()),
            ],
        };
        assert_eq!(predicate.keep(&data).expect("filter").height(), 2);
    }

    #[test]
    fn a_missing_column_is_an_error_not_an_empty_chart() {
        let data = sample();
        let predicate = Predicate::Equal {
            field: "nope".to_owned(),
            value: PredicateValue::Text("x".to_owned()),
        };
        assert!(predicate.keep(&data).is_err());
    }
}
