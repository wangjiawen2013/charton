//! Fold: several columns into a name column and a value column.
//!
//! Every input column is kept and repeated once per folded field, so the result
//! can still be grouped or coloured by anything the wide table had.
//!
//! This is the first transform that creates columns, which is why the spec layer
//! validates against transformed rows rather than the input table.
//!
//! Pure `Dataset → Dataset`; the `Chart` method is a wrapper.
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, SemanticType};
use crate::error::ChartonError;
use crate::mark::Mark;

/// Default name of the column that holds the folded column names.
pub const DEFAULT_KEY: &str = "key";

/// Default name of the column that holds the folded values.
pub const DEFAULT_VALUE: &str = "value";

/// A wide-to-long reshape.
#[derive(Debug, Clone, PartialEq)]
pub struct Fold {
    fields: Vec<String>,
    key: String,
    value: String,
}

impl Fold {
    /// Fold `fields` into two columns, named after Vega-Lite's defaults.
    ///
    /// # Errors
    ///
    /// Fails when `fields` is empty, or when it repeats a name.
    pub fn new<I, S>(fields: I) -> Result<Self, ChartonError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::with_names(fields, DEFAULT_KEY, DEFAULT_VALUE)
    }

    /// Fold `fields`, naming the two output columns explicitly.
    ///
    /// # Errors
    ///
    /// Fails when `fields` is empty, or when it repeats a name.
    pub fn with_names<I, S>(
        fields: I,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, ChartonError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let fields: Vec<String> = fields.into_iter().map(Into::into).collect();
        if fields.is_empty() {
            return Err(ChartonError::Data(
                "fold needs at least one column to fold".into(),
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for field in &fields {
            if !seen.insert(field.as_str()) {
                return Err(ChartonError::Data(format!(
                    "fold lists the column '{field}' more than once"
                )));
            }
        }
        Ok(Self {
            fields,
            key: key.into(),
            value: value.into(),
        })
    }

    /// The columns being folded.
    #[must_use]
    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    /// The two output column names.
    #[must_use]
    pub fn output_names(&self) -> (&str, &str) {
        (&self.key, &self.value)
    }

    /// The reshaped dataset.
    ///
    /// Every input column is kept and repeated once per folded field, so the
    /// result can still be grouped or coloured by anything the wide table had.
    ///
    /// The value column is continuous when every folded column is, and text
    /// otherwise: a single text column would otherwise lose its values, and a
    /// mixed column cannot be numeric without inventing a type for the text
    /// cells.
    ///
    /// # Errors
    ///
    /// Fails when a folded column does not exist, or when an output name would
    /// overwrite an existing column.
    pub fn apply(&self, data: &Dataset) -> Result<Dataset, ChartonError> {
        let height = data.height();

        if let Some(clash) = data
            .get_column_names()
            .iter()
            .find(|name| **name == self.key || **name == self.value)
        {
            return Err(ChartonError::Data(format!(
                "fold would overwrite the existing column '{clash}'; rename the output with `as`"
            )));
        }

        // Column and type checks before any allocation.
        let mut all_continuous = true;
        for field in &self.fields {
            let column = data.column(field)?;
            all_continuous &= column.semantic_type() == SemanticType::Continuous;
        }

        // Each input row becomes one row per folded field, in field order.
        let mut indices = Vec::with_capacity(height * self.fields.len());
        for row in 0..height {
            for _ in &self.fields {
                indices.push(row);
            }
        }

        let mut folded = Dataset::new();
        for name in data.get_column_names() {
            let column = data.column(&name)?;
            folded.add_column(name, column.take(&indices))?;
        }

        let mut keys = Vec::with_capacity(indices.len());
        for _ in 0..height {
            for field in &self.fields {
                keys.push(Some(field.clone()));
            }
        }
        folded.add_column(self.key.clone(), keys)?;

        folded.add_column(self.value.clone(), self.value_column(data, all_continuous)?)?;

        Ok(folded)
    }

    /// The folded values, as numbers when they are all continuous, as text
    /// otherwise.
    fn value_column(
        &self,
        data: &Dataset,
        all_continuous: bool,
    ) -> Result<ColumnVector, ChartonError> {
        if all_continuous {
            let mut values = Vec::with_capacity(data.height() * self.fields.len());
            for row in 0..data.height() {
                for field in &self.fields {
                    values.push(data.column(field)?.get(row).to_f64());
                }
            }
            return Ok(ColumnVector::from(values));
        }

        let mut values = Vec::with_capacity(data.height() * self.fields.len());
        for row in 0..data.height() {
            for field in &self.fields {
                values.push(data.column(field)?.get(row).to_string());
            }
        }
        Ok(ColumnVector::from(values))
    }
}

impl<T: Mark> Chart<T> {
    /// Folds several columns into a name column and a value column.
    ///
    /// ```rust,ignore
    /// chart!(year, north, south)?
    ///     .transform_fold(["north", "south"])?
    ///     .mark_line()?
    ///     .encode((alt::x("year"), alt::y("value"), alt::color("key")))?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails when a folded column does not exist, or when an output name would
    /// overwrite an existing column.
    pub fn transform_fold(mut self, fold: &Fold) -> Result<Self, ChartonError> {
        self.data = fold.apply(&self.data)?;
        if let Some(source) = self.source_data.take() {
            self.source_data = Some(fold.apply(&source)?);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::Fold;
    use crate::core::data::Dataset;

    fn wide() -> Dataset {
        let mut data = Dataset::new();
        data.add_column("year", vec![2020_i64, 2021])
            .expect("year column");
        data.add_column("north", vec![1.0_f64, 2.0])
            .expect("north column");
        data.add_column("south", vec![3.0_f64, 4.0])
            .expect("south column");
        data
    }

    #[test]
    fn folding_multiplies_rows_and_adds_two_columns() {
        let folded = Fold::new(["north", "south"])
            .expect("fold")
            .apply(&wide())
            .expect("apply");

        assert_eq!(folded.height(), 4);
        // The original columns survive, so the result can still be grouped.
        assert!(folded.get_column_names().contains(&"year".to_owned()));
        assert!(folded.get_column_names().contains(&"key".to_owned()));
        assert!(folded.get_column_names().contains(&"value".to_owned()));
    }

    #[test]
    fn folding_preserves_field_order() {
        let folded = Fold::with_names(["north", "south"], "metric", "amount")
            .expect("fold")
            .apply(&wide())
            .expect("apply");
        let key = folded.column("metric").expect("key column");
        assert_eq!(key.get(0).to_string().as_deref(), Some("north"));
        assert_eq!(key.get(1).to_string().as_deref(), Some("south"));
        assert_eq!(key.get(2).to_string().as_deref(), Some("north"));
    }

    #[test]
    fn a_missing_column_is_an_error() {
        let fold = Fold::new(["north", "east"]).expect("fold");
        assert!(fold.apply(&wide()).is_err());
    }

    #[test]
    fn an_output_name_may_not_overwrite_a_column() {
        let fold = Fold::with_names(["north", "south"], "year", "value").expect("fold");
        assert!(fold.apply(&wide()).is_err());
    }

    #[test]
    fn an_empty_or_repeated_field_list_is_refused() {
        assert!(Fold::new(Vec::<String>::new()).is_err());
        assert!(Fold::new(["north", "north"]).is_err());
    }
}
