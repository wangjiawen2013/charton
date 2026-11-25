//! Aggregate: collapse rows into one row per group.
//!
//! Keeps the group columns and the measures; everything else is dropped, because
//! carrying the other columns would claim one row's values stand for the group.
//!
//! Choices worth knowing:
//!
//! * groups keep first-appearance order, not sorted order;
//! * a missing group value is its own group, as on the non-positional channels;
//! * `count` needs no column;
//! * an aggregate over no rows is null, not zero — except the global count over
//!   an empty table, which is 0.
//!
//! Pure `Dataset → Dataset`; the `Chart` method is a wrapper.
use crate::chart::Chart;
use crate::core::data::{AggregateOp, ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;

/// Group keys in first-appearance order, plus the rows of each group.
type Grouped = (Vec<Vec<String>>, AHashMap<Vec<String>, Vec<usize>>);

/// One measure: what to compute, from which column, under which name.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregateField {
    /// The operation.
    pub op: AggregateOp,
    /// The column to read. `None` means "the group itself", which only `count`
    /// uses.
    pub field: Option<String>,
    /// The output column name.
    pub as_name: String,
}

impl AggregateField {
    /// A count of the group's rows, which needs no column.
    #[must_use]
    pub fn count(as_name: impl Into<String>) -> Self {
        Self {
            op: AggregateOp::Count,
            field: None,
            as_name: as_name.into(),
        }
    }

    /// An operation over a named column.
    #[must_use]
    pub fn of(op: AggregateOp, field: impl Into<String>, as_name: impl Into<String>) -> Self {
        Self {
            op,
            field: Some(field.into()),
            as_name: as_name.into(),
        }
    }
}

/// A group-by plus a set of measures.
#[derive(Debug, Clone, PartialEq)]
pub struct Aggregate {
    groupby: Vec<String>,
    fields: Vec<AggregateField>,
}

impl Aggregate {
    /// Build an aggregation.
    ///
    /// # Errors
    ///
    /// Fails when there is nothing to aggregate, or when two outputs share a
    /// name — either would lose a column silently.
    pub fn new(groupby: Vec<String>, fields: Vec<AggregateField>) -> Result<Self, ChartonError> {
        if fields.is_empty() {
            return Err(ChartonError::Data(
                "aggregate needs at least one measure".into(),
            ));
        }

        let mut seen = std::collections::HashSet::new();
        for field in &fields {
            if !seen.insert(field.as_name.as_str()) {
                return Err(ChartonError::Data(format!(
                    "aggregate writes the column '{}' more than once",
                    field.as_name
                )));
            }
            if field.op != AggregateOp::Count && field.field.is_none() {
                return Err(ChartonError::Data(format!(
                    "aggregate `{}` needs a field",
                    field.as_name
                )));
            }
        }

        Ok(Self { groupby, fields })
    }

    /// The grouping columns.
    #[must_use]
    pub fn groupby(&self) -> &[String] {
        &self.groupby
    }

    /// The measures.
    #[must_use]
    pub fn fields(&self) -> &[AggregateField] {
        &self.fields
    }

    /// The output column names, group columns first.
    #[must_use]
    pub fn output_names(&self) -> Vec<String> {
        let mut names = self.groupby.clone();
        names.extend(self.fields.iter().map(|field| field.as_name.clone()));
        names
    }

    /// The collapsed dataset.
    ///
    /// Keeps only the group columns and the measures: an aggregate that carried
    /// the original columns along would be claiming one arbitrary row's values
    /// represent the group.
    ///
    /// # Errors
    ///
    /// Fails when a group or measure column does not exist.
    pub fn apply(&self, data: &Dataset) -> Result<Dataset, ChartonError> {
        let height = data.height();

        let group_columns: Vec<&ColumnVector> = self
            .groupby
            .iter()
            .map(|field| data.column(field))
            .collect::<Result<_, _>>()?;
        let measures: Vec<Option<&ColumnVector>> = self
            .fields
            .iter()
            .map(|field| match field.field.as_ref() {
                Some(name) => data.column(name).map(Some),
                None => Ok(None),
            })
            .collect::<Result<_, _>>()?;

        let (order, groups) = self.group_rows(&group_columns, height);

        let mut aggregated = Dataset::new();
        if !self.groupby.is_empty() {
            // One representative row per group carries the group columns; a
            // group is never empty because it exists only once a row joins it.
            let representatives: Vec<usize> = order.iter().map(|key| groups[key][0]).collect();
            for (position, name) in self.groupby.iter().enumerate() {
                aggregated
                    .add_column(name.clone(), group_columns[position].take(&representatives))?;
            }
        }

        for (field, column) in self.fields.iter().zip(measures) {
            let values: Vec<Option<f64>> = order
                .iter()
                .map(|key| {
                    let rows = &groups[key];
                    match column {
                        // `count` is a property of the group.
                        None => Some(rows.len() as f64),
                        Some(column) => {
                            if rows.is_empty() {
                                return None;
                            }
                            let value = field.op.aggregate_by_index(column, rows);
                            (!value.is_nan()).then_some(value)
                        }
                    }
                })
                .collect();
            aggregated.add_column(field.as_name.clone(), values)?;
        }

        Ok(aggregated)
    }

    /// Group row indices by the group columns, in first-appearance order.
    fn group_rows(&self, group_columns: &[&ColumnVector], height: usize) -> Grouped {
        let mut groups: AHashMap<Vec<String>, Vec<usize>> = AHashMap::new();
        let mut order: Vec<Vec<String>> = Vec::new();

        if self.groupby.is_empty() {
            // No grouping: the whole table is one group, so the result is a
            // single row — and `count` reports 0 for an empty table.
            groups.insert(Vec::new(), (0..height).collect());
            order.push(Vec::new());
            return (order, groups);
        }

        for row in 0..height {
            // A missing group value is its own group rather than a merge into
            // a neighbour, matching the non-positional channel rule.
            let key: Vec<String> = group_columns
                .iter()
                .map(|column| column.label_with_missing(row))
                .collect();
            if !groups.contains_key(&key) {
                order.push(key.clone());
            }
            groups.entry(key).or_default().push(row);
        }

        (order, groups)
    }
}

impl<T: Mark> Chart<T> {
    /// Collapses the rows to one per group.
    ///
    /// ```rust,ignore
    /// let agg = Aggregate::new(
    ///     vec!["region".to_owned()],
    ///     vec![AggregateField::of(AggregateOp::Mean, "sales", "mean_sales")],
    /// )?;
    /// chart!(rows)?.transform_aggregate(&agg)?.mark_bar()?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails when a group or measure column does not exist.
    pub fn transform_aggregate(mut self, aggregate: &Aggregate) -> Result<Self, ChartonError> {
        self.data = aggregate.apply(&self.data)?;
        if let Some(source) = self.source_data.take() {
            self.source_data = Some(aggregate.apply(&source)?);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Aggregate, AggregateField};
    use crate::core::data::{AggregateOp, Dataset};

    fn table() -> Dataset {
        let mut data = Dataset::new();
        data.add_column(
            "region",
            vec!["north".to_owned(), "south".to_owned(), "north".to_owned()],
        )
        .expect("region");
        data.add_column("sales", vec![1.0_f64, 2.0, 3.0])
            .expect("sales");
        data
    }

    #[test]
    fn groups_collapse_in_first_appearance_order() {
        let aggregate = Aggregate::new(
            vec!["region".to_owned()],
            vec![AggregateField::of(AggregateOp::Sum, "sales", "total")],
        )
        .expect("aggregate");

        let result = aggregate.apply(&table()).expect("apply");
        assert_eq!(result.height(), 2);
        assert_eq!(
            result
                .column("region")
                .expect("region")
                .get(0)
                .to_string()
                .as_deref(),
            Some("north"),
            "first appearance order, not alphabetical"
        );
        assert_eq!(
            result.column("total").expect("total").get(0).to_f64(),
            Some(4.0)
        );
    }

    #[test]
    fn count_needs_no_column() {
        let aggregate =
            Aggregate::new(Vec::new(), vec![AggregateField::count("rows")]).expect("aggregate");
        let result = aggregate.apply(&table()).expect("apply");

        assert_eq!(result.height(), 1);
        assert_eq!(
            result.column("rows").expect("rows").get(0).to_f64(),
            Some(3.0)
        );
    }

    #[test]
    fn an_empty_table_still_counts_zero() {
        let empty = Dataset::new();
        let aggregate =
            Aggregate::new(Vec::new(), vec![AggregateField::count("rows")]).expect("aggregate");
        let result = aggregate.apply(&empty).expect("apply");

        assert_eq!(result.height(), 1);
        assert_eq!(
            result.column("rows").expect("rows").get(0).to_f64(),
            Some(0.0)
        );
    }

    #[test]
    fn a_missing_measure_is_an_error() {
        let aggregate = Aggregate::new(
            vec!["region".to_owned()],
            vec![AggregateField::of(AggregateOp::Sum, "nope", "total")],
        )
        .expect("aggregate");
        assert!(aggregate.apply(&table()).is_err());
    }

    #[test]
    fn duplicate_output_names_are_refused() {
        assert!(
            Aggregate::new(
                vec!["region".to_owned()],
                vec![
                    AggregateField::of(AggregateOp::Sum, "sales", "x"),
                    AggregateField::of(AggregateOp::Mean, "sales", "x"),
                ],
            )
            .is_err()
        );
    }
}
