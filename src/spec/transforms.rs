//! Applying a document's transforms to rows.
//!
//! Validation calls this to learn which rows the mark will see; lowering calls
//! it to produce them. Sharing the function is what keeps the two stages from
//! disagreeing about what a transform does.
//!
//! Malformed entries (a filter with two conditions, an empty `fold`) are skipped
//! here, because the validator already reported them with a pointer.
//! Data-dependent failures (a missing column, an output name that would
//! overwrite one) are reported here, because they need rows to discover.
use crate::core::data::{Dataset, SemanticType};
use crate::error::ChartonError;
use crate::spec::error::{SpecError, codes, preview_columns};
use crate::spec::types::semantic_name;
use crate::spec::types::{
    AggregateEntry, BinEntry, BinSpec, FilterExpr, FilterValue, FoldEntry, Spec, Transform,
    WindowEntry,
};
use crate::transform::aggregate_transform::{Aggregate, AggregateField};
use crate::transform::bin_transform::Bin;
use crate::transform::filter_transform::{Predicate, PredicateValue};
use crate::transform::fold_transform::{DEFAULT_KEY, DEFAULT_VALUE, Fold};
use crate::transform::window_transform::{WindowFieldDef, WindowTransform};

/// Apply every transform, in order.
///
/// Stops at the first failure: a later step reads what an earlier one produced,
/// so continuing after a broken step would report consequences instead of
/// causes.
pub(crate) fn apply(spec: &Spec, data: &Dataset) -> Result<Dataset, Vec<SpecError>> {
    let mut current = data.clone();
    for (index, transform) in spec.transform.iter().enumerate() {
        current = match step(transform, &current, index) {
            Ok(next) => next,
            Err(error) => return Err(vec![error]),
        };
    }
    Ok(current)
}

fn step(transform: &Transform, data: &Dataset, index: usize) -> Result<Dataset, SpecError> {
    match transform {
        Transform::Filter(entry) => filter_step(&entry.filter, data, index),
        Transform::Fold(entry) => fold_step(entry, data, index),
        Transform::Aggregate(entry) => aggregate_step(entry, data, index),
        Transform::Bin(entry) => bin_step(entry, data, index),
        Transform::Window(entry) => window_step(entry, data, index),
    }
}

// ---------------------------------------------------------------------------
// filter
// ---------------------------------------------------------------------------

fn filter_step(expr: &FilterExpr, data: &Dataset, index: usize) -> Result<Dataset, SpecError> {
    let Some(predicate) = predicate_of(expr) else {
        return Ok(data.clone()); // Malformed; already reported.
    };

    let Ok(column) = data.column(&expr.field) else {
        return Err(missing_column(
            format!("/transform/{index}/filter/field"),
            &expr.field,
            data,
        ));
    };

    if expr.range.is_some() && column.semantic_type() != SemanticType::Continuous {
        return Err(SpecError::new(
            codes::DATA_TYPE,
            format!("/transform/{index}/filter/range"),
            format!(
                "A numeric range cannot test `{}`, which is {}",
                expr.field,
                semantic_name(column.semantic_type())
            ),
        )
        .with_hint("Use `equal` or `oneOf` for a category."));
    }

    predicate.keep(data).map_err(|error| internal(index, error))
}

/// The spec predicate as the core predicate, or `None` when the document is
/// malformed and has already been reported.
fn predicate_of(expr: &FilterExpr) -> Option<Predicate> {
    let field = expr.field.clone();

    if let Some(value) = expr.equal.as_ref() {
        return Some(Predicate::Equal {
            field,
            value: predicate_value(value),
        });
    }

    if let Some([min, max]) = expr.range.as_ref() {
        return Some(Predicate::Range {
            field,
            min: min.as_number(),
            max: max.as_number(),
        });
    }

    expr.one_of.as_ref().map(|values| Predicate::OneOf {
        field,
        values: values.iter().map(predicate_value).collect(),
    })
}

fn predicate_value(value: &FilterValue) -> PredicateValue {
    match value {
        FilterValue::Number(number) => PredicateValue::Number(*number),
        FilterValue::Text(text) => PredicateValue::Text(text.clone()),
    }
}

// ---------------------------------------------------------------------------
// fold
// ---------------------------------------------------------------------------

fn fold_step(entry: &FoldEntry, data: &Dataset, index: usize) -> Result<Dataset, SpecError> {
    if entry.fold.is_empty() {
        return Ok(data.clone()); // Malformed; already reported.
    }

    for (position, field) in entry.fold.iter().enumerate() {
        if data.column(field).is_err() {
            return Err(missing_column(
                format!("/transform/{index}/fold/{position}"),
                field,
                data,
            ));
        }
    }

    let (key, value) = match entry.as_names.as_ref() {
        Some([key, value]) => (key.clone(), value.clone()),
        None => (DEFAULT_KEY.to_owned(), DEFAULT_VALUE.to_owned()),
    };

    if let Some(clash) = [&key, &value]
        .into_iter()
        .find(|name| data.get_column_names().contains(name))
    {
        let at = if entry.as_names.is_some() {
            format!("/transform/{index}/as")
        } else {
            format!("/transform/{index}/fold")
        };
        return Err(SpecError::new(
            codes::NAME_CLASH,
            at,
            format!("Folding would overwrite the existing column `{clash}`"),
        )
        .with_hint(format!(
            "Name the two output columns explicitly: \"as\": [\"{key}\", \"other\"]."
        )));
    }

    Fold::with_names(entry.fold.clone(), key, value)
        .and_then(|fold| fold.apply(data))
        .map_err(|error| internal(index, error))
}

// ---------------------------------------------------------------------------
// aggregate
// ---------------------------------------------------------------------------

fn aggregate_step(
    entry: &AggregateEntry,
    data: &Dataset,
    index: usize,
) -> Result<Dataset, SpecError> {
    if entry.aggregate.is_empty() {
        return Ok(data.clone()); // Malformed; already reported.
    }

    for (position, name) in entry.groupby.iter().enumerate() {
        if data.column(name).is_err() {
            return Err(missing_column(
                format!("/transform/{index}/groupby/{position}"),
                name,
                data,
            ));
        }
    }

    let mut fields = Vec::with_capacity(entry.aggregate.len());
    for (position, spec) in entry.aggregate.iter().enumerate() {
        let field = match spec.field.as_ref() {
            Some(name) => {
                let Ok(column) = data.column(name) else {
                    return Err(missing_column(
                        format!("/transform/{index}/aggregate/{position}/field"),
                        name,
                        data,
                    ));
                };
                // Everything but `count` is arithmetic, which a category cannot do.
                if spec.op.to_core() != crate::core::data::AggregateOp::Count
                    && column.semantic_type() != SemanticType::Continuous
                {
                    return Err(SpecError::new(
                        codes::DATA_TYPE,
                        format!("/transform/{index}/aggregate/{position}/field"),
                        format!(
                            "`{}` needs a continuous field; `{name}` is {}",
                            spec.op.as_str(),
                            semantic_name(column.semantic_type())
                        ),
                    )
                    .with_hint("Only `count` works on a categorical field."));
                }
                Some(name.clone())
            }
            None => None,
        };

        // An aggregate output may not shadow a group column: the group column is
        // still needed, and `add_column` would overwrite it.
        if entry.groupby.contains(&spec.as_name) {
            return Err(SpecError::new(
                codes::NAME_CLASH,
                format!("/transform/{index}/aggregate/{position}/as"),
                format!(
                    "`{}` would overwrite the group column of the same name",
                    spec.as_name
                ),
            )
            .with_hint("Rename the measure, or the group column."));
        }

        fields.push(AggregateField {
            op: spec.op.to_core(),
            field,
            as_name: spec.as_name.clone(),
        });
    }

    Aggregate::new(entry.groupby.clone(), fields)
        .and_then(|aggregate| aggregate.apply(data))
        .map_err(|error| internal(index, error))
}

// ---------------------------------------------------------------------------
// bin
// ---------------------------------------------------------------------------

fn bin_step(entry: &BinEntry, data: &Dataset, index: usize) -> Result<Dataset, SpecError> {
    let maxbins = match entry.bin {
        BinSpec::Enabled(true) => None,
        BinSpec::Enabled(false) => return Ok(data.clone()), // Malformed; already reported.
        BinSpec::Params(params) => {
            if params.step.is_some() || params.maxbins == Some(0) {
                return Ok(data.clone()); // Malformed; already reported.
            }
            params.maxbins
        }
    };

    let Ok(column) = data.column(&entry.field) else {
        return Err(missing_column(
            format!("/transform/{index}/field"),
            &entry.field,
            data,
        ));
    };

    if column.semantic_type() != SemanticType::Continuous {
        return Err(SpecError::new(
            codes::DATA_TYPE,
            format!("/transform/{index}/field"),
            format!(
                "`bin` needs a continuous field; `{}` is {}",
                entry.field,
                semantic_name(column.semantic_type())
            ),
        )
        .with_hint("Bin a measure, not a category."));
    }

    let (start, end) = match entry.as_names.as_ref() {
        Some([start, end]) => (start.clone(), end.clone()),
        None => Bin::default_names(&entry.field),
    };

    if let Some(clash) = [&start, &end]
        .into_iter()
        .find(|name| data.get_column_names().contains(name))
    {
        let at = if entry.as_names.is_some() {
            format!("/transform/{index}/as")
        } else {
            format!("/transform/{index}")
        };
        return Err(SpecError::new(
            codes::NAME_CLASH,
            at,
            format!("Binning would overwrite the existing column `{clash}`"),
        )
        .with_hint("Name the edges explicitly with `as`."));
    }

    Bin::new(entry.field.clone(), maxbins, start, end)
        .and_then(|bin| bin.apply(data))
        .map_err(|error| internal(index, error))
}

// ---------------------------------------------------------------------------
// window
// ---------------------------------------------------------------------------

fn window_step(entry: &WindowEntry, data: &Dataset, index: usize) -> Result<Dataset, SpecError> {
    if entry.window.is_empty() {
        return Ok(data.clone()); // Malformed; already reported.
    }

    for (position, name) in entry.groupby.iter().enumerate() {
        if data.column(name).is_err() {
            return Err(missing_column(
                format!("/transform/{index}/groupby/{position}"),
                name,
                data,
            ));
        }
    }

    // The core adds the output column in place, so a clash would overwrite real
    // data. Track the names as they become taken, including earlier outputs of
    // this same entry.
    let mut taken: Vec<String> = data.get_column_names();

    let mut current = data.clone();
    for (position, spec) in entry.window.iter().enumerate() {
        if !spec.op.is_implemented() {
            continue; // Malformed; already reported.
        }
        let Some(field) = spec.field.as_ref() else {
            continue; // Malformed; already reported.
        };
        if current.column(field).is_err() {
            return Err(missing_column(
                format!("/transform/{index}/window/{position}/field"),
                field,
                &current,
            ));
        }
        if taken.contains(&spec.as_name) {
            return Err(SpecError::new(
                codes::NAME_CLASH,
                format!("/transform/{index}/window/{position}/as"),
                format!("`{}` would overwrite an existing column", spec.as_name),
            )
            .with_hint("Rename the window output."));
        }

        let transform = WindowTransform::new(WindowFieldDef::new(
            field,
            spec.op.to_core(),
            spec.as_name.clone(),
        ))
        .with_groupbys(entry.groupby.clone());

        current = transform
            .apply(&current)
            .map_err(|error| internal(index, error))?;
        taken.push(spec.as_name.clone());
    }

    Ok(current)
}

// ---------------------------------------------------------------------------
// shared diagnostics
// ---------------------------------------------------------------------------

fn missing_column(at: String, field: &str, data: &Dataset) -> SpecError {
    let available = data.get_column_names();
    SpecError::new(
        codes::DATA_FIELD,
        at,
        format!("Column `{field}` is not in the data"),
    )
    .with_hint(format!(
        "Available columns: {}",
        preview_columns(&available)
    ))
    .with_expected(available)
}

/// A failure from the core, which validation should have prevented.
fn internal(index: usize, error: ChartonError) -> SpecError {
    SpecError::new(
        codes::INTERNAL,
        format!("/transform/{index}"),
        error.to_string(),
    )
    .with_hint("This is a bug: validation accepted a transform that cannot run.")
}
