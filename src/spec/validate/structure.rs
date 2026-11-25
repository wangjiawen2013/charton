//! The document's shape: one mark or layers, data at the root, one facet.
//!
//! Everything here is answerable from the document alone. Checks that need rows
//! live in `data.rs`.
use crate::spec::SUPPORTED_EXTENSION_MARKS;
use crate::spec::error::{SpecError, codes};
use crate::spec::types::{
    Data, ExtensionMark, Mark, SPEC_SCHEMA_URL, SUPPORTED_SPEC_VERSION, Spec, SpecMark,
};
use crate::spec::validate::{MAX_INLINE_CELLS, MAX_INLINE_ROWS};

/// The document's shape: one mark or layers, data at the root, one facet.
pub(super) fn check_structure(spec: &Spec, at: &str, is_root: bool, errors: &mut Vec<SpecError>) {
    match (is_root, spec.data.is_some()) {
        (true, false) => errors.push(
            SpecError::new(
                codes::MISSING_FIELD,
                format!("{at}/data"),
                "`data` is missing",
            )
            .with_hint("The document root names its rows with `url` or `values`."),
        ),
        (false, true) => errors.push(
            SpecError::new(
                codes::UNSUPPORTED_FIELD,
                format!("{at}/data"),
                "A layer may not bring its own data",
            )
            .with_hint("Put `data` on the root; every layer reads it."),
        ),
        _ => {}
    }

    let has_mark =
        spec.mark.is_some() || spec.charton.as_ref().is_some_and(|ext| ext.mark.is_some());
    let layered = !spec.layer.is_empty();

    if has_mark && layered {
        errors.push(
            SpecError::new(
                codes::MARK_SOURCE,
                format!("{at}/mark"),
                "A document is either one `mark` or a `layer` list, not both",
            )
            .with_hint("Remove `mark`, or wrap the layers in their own document."),
        );
    }

    if !is_root {
        let owned: &[(&str, bool, &str)] = &[
            ("/layer", layered, "Layers do not nest; flatten them."),
            ("/facet", spec.facet.is_some(), "Facets belong to the root."),
            (
                "/transform",
                !spec.transform.is_empty(),
                "Transforms belong to the root and run before the layers.",
            ),
            (
                "/title",
                spec.title.is_some(),
                "The chart title belongs to the root.",
            ),
            (
                "/description",
                spec.description.is_some(),
                "The description belongs to the root.",
            ),
        ];
        for (path, present, hint) in owned {
            if *present {
                errors.push(
                    SpecError::new(
                        codes::UNSUPPORTED_FIELD,
                        format!("{at}{path}"),
                        "A layer may not set this",
                    )
                    .with_hint(*hint),
                );
            }
        }
    }

    for (index, child) in spec.layer.iter().enumerate() {
        check_structure(child, &format!("{at}/layer/{index}"), false, errors);
    }
}

/// A facet splits by one field (wrap) or by two (grid), never both spellings.
pub(super) fn check_facet(spec: &Spec, errors: &mut Vec<SpecError>) {
    let Some(facet) = spec.facet.as_ref() else {
        return;
    };

    let wrap = facet.field.is_some();
    let grid = facet.row.is_some() || facet.column.is_some();

    match (wrap, grid) {
        (true, true) => errors.push(
            SpecError::new(
                codes::TYPE_MISMATCH,
                "/facet",
                "A facet wraps by `field` or grids by `row` and `column`, not both",
            )
            .with_hint("Keep one spelling."),
        ),
        (false, false) => errors.push(
            SpecError::new(codes::MISSING_FIELD, "/facet", "The facet names no field")
                .with_hint("Add `field`, or `row` and `column`."),
        ),
        (true, false) => {
            if facet.columns == Some(0) {
                errors.push(SpecError::new(
                    codes::TYPE_MISMATCH,
                    "/facet/columns",
                    "`columns` must be at least 1",
                ));
            }
        }
        (false, true) => {
            for (name, value) in [
                ("row", facet.row.as_ref()),
                ("column", facet.column.as_ref()),
            ] {
                if value.is_none() {
                    errors.push(
                        SpecError::new(
                            codes::MISSING_FIELD,
                            format!("/facet/{name}"),
                            format!("A grid facet needs `{name}`"),
                        )
                        .with_hint("Use `field` for a wrap layout instead."),
                    );
                }
            }
            if facet.columns.is_some() {
                errors.push(
                    SpecError::new(
                        codes::UNSUPPORTED_FIELD,
                        "/facet/columns",
                        "`columns` only applies to a wrap facet",
                    )
                    .with_hint("A grid's shape comes from `row` and `column`."),
                );
            }
        }
    }
}

pub(super) fn check_version(spec: &Spec, errors: &mut Vec<SpecError>) {
    let Some(url) = spec.schema.as_deref() else {
        return; // Absent means "the version this build reads".
    };

    if url == SPEC_SCHEMA_URL {
        return;
    }

    if url.contains("vega-lite") {
        errors.push(
            SpecError::new(
                codes::SPEC_VERSION,
                "/$schema",
                "This is Vega-Lite's schema URL, which describes more than Charton implements",
            )
            .with_hint(format!(
                "Charton validates a subset with its own schema: `{SPEC_SCHEMA_URL}`"
            ))
            .with_expected([SPEC_SCHEMA_URL]),
        );
        return;
    }

    let version = url
        .rsplit('/')
        .next()
        .and_then(|file| file.strip_prefix("charton-"))
        .and_then(|file| file.strip_suffix(".schema.json"));

    match version {
        Some(version) if version == SUPPORTED_SPEC_VERSION => {}
        Some(version) => errors.push(
            SpecError::new(
                codes::SPEC_VERSION,
                "/$schema",
                format!(
                    "Spec version `{version}` is not supported; this build reads `{SUPPORTED_SPEC_VERSION}`"
                ),
            )
            .with_hint("Upgrade Charton, or rewrite the document for the supported version."),
        ),
        None => errors.push(
            SpecError::new(codes::SPEC_VERSION, "/$schema", "Unrecognised `$schema` URL")
                .with_hint("Use Charton's schema URL; it is the only version carrier.")
                .with_expected([SPEC_SCHEMA_URL]),
        ),
    }
}

/// Which mark the document asks for, if it asks for exactly one.
pub(super) fn check_mark_source(spec: &Spec, errors: &mut Vec<SpecError>) -> Option<SpecMark> {
    if let Some(mark) = spec.resolved_mark() {
        return Some(mark);
    }

    let both = spec.mark.is_some() && spec.charton.as_ref().is_some_and(|ext| ext.mark.is_some());
    errors.push(if both {
        SpecError::new(
            codes::MARK_SOURCE,
            "/mark",
            "Two competing marks: both `mark` and `charton.mark` are set",
        )
        .with_hint("Keep one. `mark` is Vega-Lite's; `charton.mark` is for Charton-only marks.")
    } else {
        SpecError::new(codes::MARK_SOURCE, "/mark", "No mark to draw")
            .with_hint("Add `mark` (Vega-Lite name) or `charton.mark` (Charton-only mark).")
    });
    None
}

pub(super) fn check_data(data: Option<&Data>, errors: &mut Vec<SpecError>) {
    let Some(data) = data else {
        return; // Missing root data is reported by `check_structure`.
    };
    match (&data.url, &data.values) {
        (Some(_), Some(_)) => errors.push(
            SpecError::new(
                codes::DATA_SOURCE,
                "/data",
                "`data` has both `url` and `values`",
            )
            .with_hint("Keep one source: a file path, or inline rows."),
        ),
        (None, None) => errors.push(
            SpecError::new(codes::DATA_SOURCE, "/data", "`data` names no source")
                .with_hint("Add `url` (a CSV or NDJSON path) or `values` (inline rows)."),
        ),
        (Some(url), None) => {
            if url.trim().is_empty() {
                errors.push(
                    SpecError::new(codes::DATA_SOURCE, "/data/url", "`url` is empty")
                        .with_hint("Point it at a CSV or NDJSON file."),
                );
            } else if url.contains("://") {
                errors.push(
                    SpecError::new(
                        codes::DATA_SOURCE,
                        "/data/url",
                        "Remote URLs are not read; `url` is a local file path",
                    )
                    .with_hint(
                        "Download the data first. Charton never fetches over the network from a spec.",
                    ),
                );
            }
        }
        (None, Some(rows)) => {
            if rows.is_empty() {
                errors.push(
                    SpecError::new(codes::DATA_SOURCE, "/data/values", "`values` is empty")
                        .with_hint("Provide at least one row, or use `url`."),
                );
                return;
            }
            if rows.len() > MAX_INLINE_ROWS {
                errors.push(
                    SpecError::new(
                        codes::DATA_SOURCE,
                        "/data/values",
                        format!(
                            "{} inline rows exceeds the limit of {MAX_INLINE_ROWS}",
                            rows.len()
                        ),
                    )
                    .with_hint("Put the data in a CSV file and use `url`."),
                );
                return;
            }

            let cells: usize = rows.iter().map(std::collections::BTreeMap::len).sum();
            if cells > MAX_INLINE_CELLS {
                errors.push(
                    SpecError::new(
                        codes::DATA_SOURCE,
                        "/data/values",
                        format!("{cells} inline cells exceeds the limit of {MAX_INLINE_CELLS}"),
                    )
                    .with_hint("Put the data in a CSV file and use `url`."),
                );
            }

            if let Some(index) = rows.iter().position(std::collections::BTreeMap::is_empty) {
                errors.push(
                    SpecError::new(
                        codes::DATA_SOURCE,
                        format!("/data/values/{index}"),
                        "Inline row has no columns",
                    )
                    .with_hint("Every row must name at least one column."),
                );
            }
        }
    }
}

/// Mark properties the schema accepts but this build cannot apply yet.
pub(super) fn check_mark_properties(spec: &Spec, errors: &mut Vec<SpecError>) {
    let Some(Mark::Def(def)) = spec.mark.as_ref() else {
        return;
    };
    for (field, set) in [
        ("size", def.size.is_some()),
        ("opacity", def.opacity.is_some()),
    ] {
        if set {
            errors.push(
                SpecError::new(
                    codes::UNSUPPORTED_FIELD,
                    format!("/mark/{field}"),
                    format!("Fixed mark `{field}` is not implemented yet"),
                )
                .with_hint("Encode it, or leave it out; mark `color` is supported."),
            );
        }
    }
}

/// Refuse a mark the dialect knows but this build cannot draw.
///
/// Saying "unsupported" here rather than failing later keeps one invariant: a
/// document that validates is a document that renders.
pub(super) fn check_mark_supported(spec: &Spec, errors: &mut Vec<SpecError>) {
    let Some(SpecMark::Extension(mark)) = spec.resolved_mark() else {
        return;
    };
    if SUPPORTED_EXTENSION_MARKS.contains(&mark) {
        return;
    }
    errors.push(
        SpecError::new(
            codes::UNSUPPORTED_FIELD,
            "/charton/mark",
            format!(
                "Charton mark `{}` is part of the dialect but is not drawable by this build",
                mark.as_str()
            ),
        )
        .with_hint(format!(
            "Supported Charton marks: {}.",
            if SUPPORTED_EXTENSION_MARKS.is_empty() {
                "none yet".to_owned()
            } else {
                SUPPORTED_EXTENSION_MARKS
                    .iter()
                    .map(|mark| mark.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )),
    );
}

/// `charton.z` exists for one mark and one mark only.
pub(super) fn check_extension_scalar(spec: &Spec, errors: &mut Vec<SpecError>) {
    let scalar = spec.scalar_field();
    let mark = spec.resolved_mark();

    match (mark, scalar) {
        (Some(SpecMark::Extension(ExtensionMark::Contour)), None) => errors.push(
            SpecError::new(
                codes::MISSING_FIELD,
                "/charton/z",
                "`contour` needs a scalar field",
            )
            .with_hint("Add `\"charton\": {\"mark\": \"contour\", \"z\": \"<column>\"}`."),
        ),
        (Some(SpecMark::Extension(ExtensionMark::Contour)), Some("")) => errors.push(
            SpecError::new(codes::MISSING_FIELD, "/charton/z", "`charton.z` is empty")
                .with_hint("Name the column that holds the height of the surface."),
        ),
        (Some(mark), Some(_)) if mark != SpecMark::Extension(ExtensionMark::Contour) => errors
            .push(
                SpecError::new(
                    codes::UNKNOWN_FIELD,
                    "/charton/z",
                    format!("`{}` has no scalar field", mark.as_str()),
                )
                .with_hint("`charton.z` is only read by `contour`."),
            ),
        _ => {}
    }
}
