//! Reading and checking a spec.
//!
//! Three stages, each in its own file:
//!
//! 1. **Shape** ([`from_json`]) — turn JSON into [`Spec`]. The paths come from
//!    the deserialiser, so a diagnostic names the real node.
//! 2. **Meaning** ([`validate`]) — everything checkable without rows: version,
//!    mark, channels, transforms, facet, and each layer's inherited encoding.
//! 3. **Data** ([`validate_with_data`]) — field existence, type compatibility,
//!    and whether a mark's implicit statistics would change the document.
//!
//! Validation always runs before lowering: every diagnostic points at a JSON
//! Pointer into the original document, which only exists before the spec becomes
//! a chart.
//!
//! | File | Holds |
//! |---|---|
//! | `structure.rs` | the document's shape, the facet, the data block |
//! | `channels.rs` | what a mark needs, and what a channel can carry |
//! | `transforms.rs` | one check per transform entry |
//! | `data.rs` | the checks that need rows |
//! | `messages.rs` | serde errors turned into located diagnostics |

use crate::spec::error::{ErrorReport, SpecError};
use crate::spec::types::{Encoding, Spec};

mod channels;
mod data;
mod messages;
mod structure;
mod transforms;

pub use channels::{MarkRequirements, mark_requirements};
pub use data::validate_with_data;

use channels::check_channels;
use messages::structural;
use structure::{
    check_data, check_extension_scalar, check_facet, check_mark_properties, check_mark_source,
    check_mark_supported, check_structure, check_version,
};
use transforms::check_transforms;

/// Largest inline dataset accepted, in rows.
///
/// Inline data is for reproducible examples, not for shipping a table inside a
/// JSON file. Beyond this, point `data.url` at the file.
pub const MAX_INLINE_ROWS: usize = 10_000;

/// Largest inline dataset accepted, in cells (rows × columns).
///
/// Rows alone are not enough: ten thousand rows of two columns is a demo, ten
/// thousand rows of fifty columns is a spreadsheet.
pub const MAX_INLINE_CELLS: usize = 200_000;

/// Parse and fully validate a spec document.
///
/// # Errors
///
/// Structural failures yield a single diagnostic; semantic failures yield all
/// of them.
pub fn from_json(json: &str) -> Result<Spec, Vec<SpecError>> {
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let spec = decode(&mut deserializer)?;
    finish(spec)
}

/// Parse and fully validate an already-parsed JSON value.
///
/// # Errors
///
/// As [`from_json`].
pub fn from_value(value: &serde_json::Value) -> Result<Spec, Vec<SpecError>> {
    let spec = decode(value)?;
    finish(spec)
}

/// Parse and validate, returning the JSON envelope the CLI prints.
///
/// This never fails: a broken document is a value, not a panic and not an
/// `Err` the caller has to remember to serialise.
#[must_use]
pub fn check(json: &str) -> ErrorReport {
    match from_json(json) {
        Ok(_) => ErrorReport::ok(),
        Err(errors) => ErrorReport::failed(errors),
    }
}

/// Validate a spec that has already been parsed.
///
/// Returns every problem found, in document order.
#[must_use]
pub fn validate(spec: &Spec) -> Vec<SpecError> {
    let mut errors = Vec::new();
    check_version(spec, &mut errors);
    check_structure(spec, "", true, &mut errors);
    check_data(spec.data.as_ref(), &mut errors);
    check_transforms(spec, &mut errors);
    check_facet(spec, &mut errors);

    for (leaf, encoding) in leaves(spec) {
        let mark = check_mark_source(leaf, &mut errors);
        check_mark_properties(leaf, &mut errors);
        check_mark_supported(leaf, &mut errors);
        check_extension_scalar(leaf, &mut errors);
        if let Some(mark) = mark {
            check_channels(mark, &encoding, &mut errors);
        }
    }

    errors
}

/// Every single-mark document in the tree, with its effective encoding.
///
/// Layers inherit `encoding` channel by channel, so validation and lowering both
/// ask this question and get the same answer from the same code.
pub(super) fn leaves(spec: &Spec) -> Vec<(&Spec, Encoding)> {
    let mut collected = Vec::new();
    collect_leaves(spec, None, &mut collected);
    collected
}

pub(super) fn collect_leaves<'a>(
    spec: &'a Spec,
    inherited: Option<&Encoding>,
    collected: &mut Vec<(&'a Spec, Encoding)>,
) {
    let mut effective = spec.encoding.clone();
    if let Some(parent) = inherited {
        effective.inherit(parent);
    }

    if spec.layer.is_empty() {
        collected.push((spec, effective));
    } else {
        for child in &spec.layer {
            collect_leaves(child, Some(&effective), collected);
        }
    }
}

pub(super) fn finish(spec: Spec) -> Result<Spec, Vec<SpecError>> {
    let errors = validate(&spec);
    if errors.is_empty() {
        Ok(spec)
    } else {
        Err(errors)
    }
}

pub(super) fn decode<'de, D>(deserializer: D) -> Result<Spec, Vec<SpecError>>
where
    D: serde::Deserializer<'de, Error = serde_json::Error>,
{
    serde_path_to_error::deserialize(deserializer).map_err(|err| vec![structural(&err)])
}

// ---------------------------------------------------------------------------
// Stage 1: turning serde's message into a diagnostic a machine can act on
// ---------------------------------------------------------------------------
