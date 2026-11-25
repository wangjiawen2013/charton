//! One check per transform entry: complete, unambiguous, self-consistent.
//!
//! Document-level only. Whether the columns exist, and whether the rows can
//! serve the operation, is answered by `spec::transforms::apply` when the data
//! is available.

use crate::spec::error::{SpecError, codes};
use crate::spec::types::{
    AggregateEntry, AggregateSpec, BinEntry, BinSpec, FilterExpr, FoldEntry, Spec, Transform,
    WindowEntry, WindowOpSpec,
};

/// Each transform entry must be complete and self-consistent.
///
/// Document-level only: whether the columns exist is a data question and lives
/// in `transforms::apply`. Because `Transform` is a sum type, "an entry that is
/// no transform at all" cannot reach here — that is a shape error and was
/// reported while parsing.
pub(super) fn check_transforms(spec: &Spec, errors: &mut Vec<SpecError>) {
    for (index, transform) in spec.transform.iter().enumerate() {
        let entry = format!("/transform/{index}");
        match transform {
            Transform::Filter(inner) => {
                check_filter(&inner.filter, &format!("{entry}/filter"), errors);
            }
            Transform::Fold(inner) => check_fold(inner, &entry, errors),
            Transform::Aggregate(inner) => check_aggregate(inner, &entry, errors),
            Transform::Bin(inner) => check_bin(inner, &entry, errors),
            Transform::Window(inner) => check_window(inner, &entry, errors),
        }
    }
}

/// A filter names one column and one condition.
pub(super) fn check_filter(filter: &FilterExpr, at: &str, errors: &mut Vec<SpecError>) {
    if filter.field.trim().is_empty() {
        errors.push(
            SpecError::new(
                codes::MISSING_FIELD,
                format!("{at}/field"),
                "`field` is empty",
            )
            .with_hint("Name the column the filter reads."),
        );
    }

    let conditions = [
        filter.equal.is_some(),
        filter.range.is_some(),
        filter.one_of.is_some(),
    ];
    match conditions.iter().filter(|used| **used).count() {
        1 => {}
        0 => errors.push(
            SpecError::new(codes::MISSING_FIELD, at, "Filter has no condition")
                .with_hint("Add one of `equal`, `range` or `oneOf`."),
        ),
        _ => errors.push(
            SpecError::new(
                codes::TYPE_MISMATCH,
                at,
                "A filter takes one condition: `equal`, `range` or `oneOf`",
            )
            .with_hint("Split it into two entries in the `transform` array."),
        ),
    }

    if let Some(range) = filter.range.as_ref()
        && range.iter().any(|bound| bound.as_number().is_none())
    {
        errors.push(
            SpecError::new(
                codes::TYPE_MISMATCH,
                format!("{at}/range"),
                "A range needs two numbers",
            )
            .with_hint("Use `[min, max]`, for example `[0, 10]`."),
        );
    }
}

/// A fold lists at least one column, each once, and names two distinct outputs.
pub(super) fn check_fold(entry: &FoldEntry, at: &str, errors: &mut Vec<SpecError>) {
    if entry.fold.is_empty() {
        errors.push(
            SpecError::new(
                codes::MISSING_FIELD,
                format!("{at}/fold"),
                "`fold` is empty",
            )
            .with_hint("List at least one column to fold."),
        );
    }

    if let Some(repeated) = repeated(&entry.fold) {
        errors.push(
            SpecError::new(
                codes::TYPE_MISMATCH,
                format!("{at}/fold"),
                format!("`{repeated}` is listed twice"),
            )
            .with_hint("Fold each column once."),
        );
    }

    if let Some([key, value]) = entry.as_names.as_ref() {
        if key.trim().is_empty() || value.trim().is_empty() {
            errors.push(
                SpecError::new(
                    codes::MISSING_FIELD,
                    format!("{at}/as"),
                    "`as` has an empty name",
                )
                .with_hint("Both output columns need a name."),
            );
        } else if key == value {
            errors.push(
                SpecError::new(
                    codes::NAME_CLASH,
                    format!("{at}/as"),
                    "`as` would create two columns with the same name",
                )
                .with_hint("Give the key and the value columns different names."),
            );
        }
    }
}

/// An aggregate needs measures, and each measure needs an operation and a name.
pub(super) fn check_aggregate(entry: &AggregateEntry, at: &str, errors: &mut Vec<SpecError>) {
    if entry.aggregate.is_empty() {
        errors.push(
            SpecError::new(
                codes::MISSING_FIELD,
                format!("{at}/aggregate"),
                "`aggregate` is empty",
            )
            .with_hint(
                "List at least one measure, for example {\"op\": \"mean\", \"field\": \"x\", \"as\": \"avg\"}.",
            ),
        );
    }

    if let Some(repeated) = repeated(&entry.groupby) {
        errors.push(
            SpecError::new(
                codes::TYPE_MISMATCH,
                format!("{at}/groupby"),
                format!("`{repeated}` is grouped twice"),
            )
            .with_hint("List each group column once."),
        );
    }

    let mut names: Vec<&str> = Vec::new();
    for (position, spec) in entry.aggregate.iter().enumerate() {
        let measure = format!("{at}/aggregate/{position}");

        if spec.as_name.trim().is_empty() {
            errors.push(
                SpecError::new(
                    codes::MISSING_FIELD,
                    format!("{measure}/as"),
                    "`as` is empty",
                )
                .with_hint("Name the output column."),
            );
        } else if names.contains(&spec.as_name.as_str()) || entry.groupby.contains(&spec.as_name) {
            errors.push(
                SpecError::new(
                    codes::NAME_CLASH,
                    format!("{measure}/as"),
                    format!("`{}` is already a column of the result", spec.as_name),
                )
                .with_hint("Rename the measure."),
            );
        } else {
            names.push(spec.as_name.as_str());
        }

        if spec.op != AggregateSpec::Count
            && spec
                .field
                .as_ref()
                .is_none_or(|name| name.trim().is_empty())
        {
            errors.push(
                SpecError::new(
                    codes::MISSING_FIELD,
                    format!("{measure}/field"),
                    format!("`{}` needs a field", spec.op.as_str()),
                )
                .with_hint("Only `count` is complete without a field."),
            );
        }
    }
}

/// A bin entry asks for a resolution and names two distinct outputs.
pub(super) fn check_bin(entry: &BinEntry, at: &str, errors: &mut Vec<SpecError>) {
    match entry.bin {
        BinSpec::Enabled(false) => errors.push(
            SpecError::new(
                codes::UNSUPPORTED_FIELD,
                format!("{at}/bin"),
                "`bin: false` bins nothing",
            )
            .with_hint("Remove the entry instead."),
        ),
        BinSpec::Enabled(true) => {}
        BinSpec::Params(params) => {
            if params.step.is_some() {
                errors.push(
                    SpecError::new(
                        codes::UNSUPPORTED_FIELD,
                        format!("{at}/bin/step"),
                        "A fixed bin `step` is not supported",
                    )
                    .with_hint("Use `maxbins`, and let the data choose the width."),
                );
            }
            if params.maxbins == Some(0) {
                errors.push(SpecError::new(
                    codes::TYPE_MISMATCH,
                    format!("{at}/bin/maxbins"),
                    "`maxbins` must be at least 1",
                ));
            }
        }
    }

    if entry.field.trim().is_empty() {
        errors.push(
            SpecError::new(
                codes::MISSING_FIELD,
                format!("{at}/field"),
                "`field` is empty",
            )
            .with_hint("Name the continuous column to bin."),
        );
    }

    if let Some([start, end]) = entry.as_names.as_ref() {
        if start.trim().is_empty() || end.trim().is_empty() {
            errors.push(
                SpecError::new(
                    codes::MISSING_FIELD,
                    format!("{at}/as"),
                    "`as` has an empty name",
                )
                .with_hint("Both edge columns need a name."),
            );
        } else if start == end {
            errors.push(
                SpecError::new(
                    codes::NAME_CLASH,
                    format!("{at}/as"),
                    "`as` would write the same column twice",
                )
                .with_hint("Name the start and the end differently."),
            );
        }
    }
}

/// A window entry names implemented operations, each reading a field, each
/// writing a fresh column.
pub(super) fn check_window(entry: &WindowEntry, at: &str, errors: &mut Vec<SpecError>) {
    if entry.window.is_empty() {
        errors.push(
            SpecError::new(
                codes::MISSING_FIELD,
                format!("{at}/window"),
                "`window` is empty",
            )
            .with_hint("List at least one window function."),
        );
    }

    if let Some(repeated) = repeated(&entry.groupby) {
        errors.push(
            SpecError::new(
                codes::TYPE_MISMATCH,
                format!("{at}/groupby"),
                format!("`{repeated}` is grouped twice"),
            )
            .with_hint("List each group column once."),
        );
    }

    let mut names: Vec<&str> = Vec::new();
    for (position, spec) in entry.window.iter().enumerate() {
        let function = format!("{at}/window/{position}");

        if !spec.op.is_implemented() {
            errors.push(
                SpecError::new(
                    codes::UNSUPPORTED_FIELD,
                    format!("{function}/op"),
                    format!(
                        "Window operation `{}` is recognised but not implemented",
                        spec.op.as_str()
                    ),
                )
                .with_hint(format!(
                    "Implemented operations: {}.",
                    WindowOpSpec::IMPLEMENTED
                        .iter()
                        .map(|op| op.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            );
            // The field requirement depends on the operation, so stop here.
            continue;
        }

        // Every implemented operation orders its window by a field, even
        // `row_number`: the core has no "input order" mode to fall back on.
        if spec
            .field
            .as_ref()
            .is_none_or(|name| name.trim().is_empty())
        {
            errors.push(
                SpecError::new(
                    codes::MISSING_FIELD,
                    format!("{function}/field"),
                    format!("`{}` needs a field", spec.op.as_str()),
                )
                .with_hint("The window is ordered by this column."),
            );
        }

        if spec.as_name.trim().is_empty() {
            errors.push(
                SpecError::new(
                    codes::MISSING_FIELD,
                    format!("{function}/as"),
                    "`as` is empty",
                )
                .with_hint("Name the output column."),
            );
        } else if names.contains(&spec.as_name.as_str()) {
            errors.push(
                SpecError::new(
                    codes::NAME_CLASH,
                    format!("{function}/as"),
                    format!("`{}` is written twice", spec.as_name),
                )
                .with_hint("Give each window function its own output column."),
            );
        } else {
            names.push(spec.as_name.as_str());
        }
    }
}

/// The first name that appears more than once, if any.
pub(super) fn repeated(names: &[String]) -> Option<&str> {
    let mut seen = std::collections::HashSet::new();
    names
        .iter()
        .find(|name| !seen.insert(name.as_str()))
        .map(String::as_str)
}
