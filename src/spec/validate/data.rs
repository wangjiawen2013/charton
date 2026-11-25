//! The checks that need rows.
//!
//! [`validate_with_data`] applies the transforms first, so every check below
//! looks at the table the mark will read. A filter can therefore make an
//! otherwise ambiguous bar chart valid, and a column created by `fold` is
//! visible to the encoding.
use crate::core::data::{ColumnVector, Dataset, SemanticType};
use crate::spec::error::{SpecError, codes};
use crate::spec::types::AggregateSpec;
use crate::spec::types::{
    DataType, Encoding, MarkType, ScaleTypeSpec, Spec, SpecMark, StackKind, StackSpec,
    declared_name, semantic_name,
};
use crate::spec::validate::channels::effective_scale;
use crate::spec::validate::{leaves, validate};

/// Validate against the actual rows.
///
/// This is the second tier: everything in [`validate`] plus the checks that are
/// impossible without data — whether a field exists, whether its values can
/// serve the channel, and whether the mark's implicit statistics would change
/// what the spec says.
///
/// A missing column is reported once per channel that names it, so one round
/// trip shows every place that needs changing.
#[must_use]
pub fn validate_with_data(spec: &Spec, data: &Dataset) -> Vec<SpecError> {
    let mut errors = validate(spec);

    // Everything below inspects the rows the mark will actually see, so the
    // transforms run first. They are the same pure functions lowering calls;
    // nothing is modelled twice.
    let effective = match crate::spec::transforms::apply(spec, data) {
        Ok(effective) => effective,
        Err(transform_errors) => {
            errors.extend(transform_errors);
            return errors;
        }
    };

    for (leaf, encoding) in leaves(spec) {
        check_fields(&encoding, &effective, &mut errors);
        check_channel_semantics(&encoding, &effective, &mut errors);
        check_scalar_field(leaf, &effective, &mut errors);
        check_bar_fidelity(leaf, &encoding, &effective, &mut errors);
    }
    check_facet_fields(spec, &effective, &mut errors);

    errors
}

/// The facet and contour fields must name real columns.
pub(super) fn check_facet_fields(spec: &Spec, data: &Dataset, errors: &mut Vec<SpecError>) {
    let Some(facet) = spec.facet.as_ref() else {
        return;
    };
    let fields = [
        ("field", facet.field.as_ref()),
        ("row", facet.row.as_ref()),
        ("column", facet.column.as_ref()),
    ];
    for (name, field) in fields {
        let Some(field) = field else { continue };
        if data.column(field).is_err() {
            let available = data.get_column_names();
            errors.push(
                SpecError::new(
                    codes::DATA_FIELD,
                    format!("/facet/{name}"),
                    format!("Column `{field}` is not in the data"),
                )
                .with_hint(format!(
                    "Available columns: {}",
                    crate::spec::error::preview_columns(&available)
                ))
                .with_expected(available),
            );
        }
    }
}

pub(super) fn check_fields(encoding: &Encoding, data: &Dataset, errors: &mut Vec<SpecError>) {
    let available = data.get_column_names();

    for (channel, def) in encoding.present() {
        let Ok(column) = data.column(&def.field) else {
            errors.push(
                SpecError::new(
                    codes::DATA_FIELD,
                    format!("/encoding/{channel}/field"),
                    format!("Column `{}` is not in the data", def.field),
                )
                .with_hint(format!(
                    "Available columns: {}",
                    crate::spec::error::preview_columns(&available)
                ))
                .with_expected(available.clone()),
            );
            continue;
        };

        let semantic = column.semantic_type();
        let usable = match def.data_type {
            Some(DataType::Quantitative) => SemanticType::Continuous,
            Some(DataType::Nominal | DataType::Ordinal) => SemanticType::Discrete,
            Some(DataType::Temporal) => SemanticType::Temporal,
            None => semantic,
        };

        if let Some(declared) = def.data_type
            && usable != semantic
        {
            errors.push(
                SpecError::new(
                    codes::DATA_TYPE,
                    format!("/encoding/{channel}/type"),
                    format!(
                        "Field `{}` is declared `{}` but the data is {}",
                        def.field,
                        declared_name(declared),
                        semantic_name(semantic)
                    ),
                )
                .with_hint("Remove `type` to let Charton infer it, or fix the declaration."),
            );
            continue;
        }

        if def.bin.is_some() && semantic != SemanticType::Continuous {
            errors.push(
                SpecError::new(
                    codes::DATA_TYPE,
                    format!("/encoding/{channel}/bin"),
                    format!(
                        "`bin` needs a continuous field; `{}` is {}",
                        def.field,
                        semantic_name(semantic)
                    ),
                )
                .with_hint("Bin a measure, not a category."),
            );
        }

        if let Some(aggregate) = def.aggregate
            && !matches!(aggregate, AggregateSpec::Count)
            && semantic != SemanticType::Continuous
        {
            errors.push(
                SpecError::new(
                    codes::DATA_TYPE,
                    format!("/encoding/{channel}/aggregate"),
                    format!(
                        "`{}` needs a continuous field; `{}` is {}",
                        aggregate.as_str(),
                        def.field,
                        semantic_name(semantic)
                    ),
                )
                .with_hint("Only `count` works on a categorical field."),
            );
        }
    }
}

/// `charton.z` is not a channel, but it is a column like any other.
pub(super) fn check_scalar_field(spec: &Spec, data: &Dataset, errors: &mut Vec<SpecError>) {
    let Some(scalar) = spec.scalar_field() else {
        return;
    };
    if data.column(scalar).is_err() {
        let available = data.get_column_names();
        errors.push(
            SpecError::new(
                codes::DATA_FIELD,
                "/charton/z",
                format!("Column `{scalar}` is not in the data"),
            )
            .with_hint(format!(
                "Available columns: {}",
                crate::spec::error::preview_columns(&available)
            ))
            .with_expected(available),
        );
    }
}

/// Channels whose meaning rules out some kinds of data.
///
/// These are the rules the renderer enforces anyway; checking them here is what
/// turns a mid-render `ChartonError` into a located, actionable diagnostic.
pub(super) fn check_channel_semantics(
    encoding: &Encoding,
    data: &Dataset,
    errors: &mut Vec<SpecError>,
) {
    for (channel, def) in encoding.present() {
        let Ok(column) = data.column(&def.field) else {
            continue; // Missing column is already reported.
        };
        let semantic = column.semantic_type();

        // `size` is a magnitude: a category has no area to map.
        if channel == "size" && semantic != SemanticType::Continuous {
            errors.push(
                SpecError::new(
                    codes::DATA_TYPE,
                    format!("/encoding/{channel}/field"),
                    format!(
                        "`size` needs a continuous field; `{}` is {}",
                        def.field,
                        semantic_name(semantic)
                    ),
                )
                .with_hint("Map the measure to `size`, or the category to `shape` or `color`."),
            );
        }

        // `shape` is a finite set of symbols: a continuous field has no
        // meaningful set of shapes to choose from.
        if channel == "shape" && semantic != SemanticType::Discrete {
            errors.push(
                SpecError::new(
                    codes::DATA_TYPE,
                    format!("/encoding/{channel}/field"),
                    format!(
                        "`shape` needs a categorical field; `{}` is {}",
                        def.field,
                        semantic_name(semantic)
                    ),
                )
                .with_hint("Map a category to `shape`, or the measure to `size`."),
            );
        }

        // The scale family has to be able to read the column at all.
        if let Some(kind) = effective_scale(def) {
            let required = match kind {
                ScaleTypeSpec::Linear | ScaleTypeSpec::Log => Some(SemanticType::Continuous),
                ScaleTypeSpec::Time | ScaleTypeSpec::Utc => Some(SemanticType::Temporal),
                ScaleTypeSpec::Band | ScaleTypeSpec::Point | ScaleTypeSpec::Ordinal => None,
            };
            if let Some(required) = required
                && semantic != required
            {
                errors.push(
                    SpecError::new(
                        codes::DATA_TYPE,
                        format!("/encoding/{channel}/scale/type"),
                        format!(
                            "A `{}` scale cannot read `{}`, which is {}",
                            kind.as_str(),
                            def.field,
                            semantic_name(semantic)
                        ),
                    )
                    .with_hint("Drop `scale.type` and let the data choose, or use a scale family that fits."),
                );
            }
        }
    }
}

/// Bar and area aggregate their rows; a spec may not silently get that for free.
///
/// Vega-Lite draws one bar per row when `aggregate` is absent and the bars are
/// not stacked. Charton's bar layout sums each `(x, colour)` cell. Where the two
/// would differ — repeated x values that no stack absorbs — this is an error,
/// not a different-looking chart. Everywhere else the results are identical and
/// nothing is reported.
pub(super) fn check_bar_fidelity(
    spec: &Spec,
    encoding: &Encoding,
    data: &Dataset,
    errors: &mut Vec<SpecError>,
) {
    let Some(SpecMark::Vega(MarkType::Bar | MarkType::Area)) = spec.resolved_mark() else {
        return;
    };
    let (Some(y), Some(x)) = (encoding.y.as_ref(), encoding.x.as_ref()) else {
        return; // Missing channels are already reported.
    };
    if y.aggregate.is_some() {
        return;
    }

    // A stack absorbs repeated cells: stacked bars per (x, colour) sum to the
    // same column height either way, so the two tools agree.
    let stacked = encoding.color.is_some()
        && !matches!(
            y.stack,
            Some(StackSpec::Mode(StackKind::None) | StackSpec::Flag(false))
        );
    if stacked {
        return;
    }

    let Ok(column) = data.column(&x.field) else {
        return; // Missing column is already reported.
    };

    // Each facet panel is its own chart, so repetition is judged inside a panel:
    // the same month in two regions is not a repeated x.
    let panels: Vec<&ColumnVector> = spec
        .facet
        .as_ref()
        .map(facet_fields)
        .unwrap_or_default()
        .iter()
        .filter_map(|field| data.column(field).ok())
        .collect();

    let mut seen = std::collections::HashSet::new();
    for row in 0..data.height() {
        let mut key = String::new();
        for panel in &panels {
            key.push_str(&panel.label_with_missing(row));
            key.push('\u{1f}');
        }
        key.push_str(&column.get(row).to_string().unwrap_or_default());
        if !seen.insert(key) {
            errors.push(
                SpecError::new(
                    codes::DATA_SHAPE,
                    "/encoding/y/aggregate",
                    format!(
                        "`{}` repeats, and {} without `aggregate` would sum those rows, not draw one mark per row",
                        x.field,
                        spec.resolved_mark().map_or("the mark", SpecMark::as_str)
                    ),
                )
                .with_hint(
                    "Add an `aggregate` to `y` (for example \"sum\"), or aggregate the data before writing the spec.",
                ),
            );
            return;
        }
    }
}

/// The fields a facet splits by, in panel order.
pub(super) fn facet_fields(facet: &crate::spec::FacetDef) -> Vec<&str> {
    let mut fields = Vec::new();
    if let Some(field) = facet.field.as_deref() {
        fields.push(field);
    }
    if let Some(row) = facet.row.as_deref() {
        fields.push(row);
    }
    if let Some(column) = facet.column.as_deref() {
        fields.push(column);
    }
    fields
}
