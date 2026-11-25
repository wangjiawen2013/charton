//! The JSON chart spec — a subset of [Vega-Lite v5].
//!
//! A chart can be written as a document, validated, stored next to its data and
//! replayed later. Rust users keep the typed [`crate::chart::Chart`] API; this
//! layer is an optional feature and never the required path.
//!
//! ```text
//! JSON ──deserialize──> Spec ──validate──> lower ──> Chart ──> SVG/PNG/PDF
//! ```
//!
//! # Rules for changing this module
//!
//! 1. **Validate before lowering.** Diagnostics point into the original
//!    document; keeping that true after lowering would need a reverse map from
//!    every core object back to its JSON path.
//! 2. **If it validates, it renders.** `SUPPORTED_EXTENSION_MARKS` sits next to
//!    the dispatch in `lower.rs`; adding an arm is the only way a mark becomes
//!    legal.
//! 3. **`E_INTERNAL` is never normal.** It means a check is missing. Every
//!    occurrence gets that check and a negative test.
//! 4. **Transforms are pure `Dataset → Dataset` functions, and validation runs
//!    them too** ([`transforms::apply`]). The schema after a transform is the
//!    columns of the dataset it returns; do not model it a second time.
//! 5. **A reused Vega-Lite field keeps Vega-Lite's meaning.** A difference is
//!    either reported or listed in `docs/src/spec/lowering-map.md`.
//! 6. **Charton-only capabilities live under `charton`.** No Charton mark name
//!    is accepted in `mark`, and no Charton field borrows a Vega-Lite name.
//! 7. **A spec is data, not a program.** No expression strings, no `calculate`,
//!    no closures, no file or network access from a document.
//! 8. **Structural errors are reported one at a time; semantic errors all at
//!    once.** A document that cannot be typed has no trustworthy later fields.
//! 9. **Codes are stable, wording is not**, and every code must be reachable
//!    from the negative corpus.
//! 10. **"Does not exist" and "not implemented" are different answers.**
//!     `E_UNKNOWN_*` means fix the spelling; `E_UNSUPPORTED_*` means valid
//!     Vega-Lite that Charton does not do.
//! 11. **Never overwrite a column.** A transform output or an encoding field
//!     that collides with a column is `E_NAME_CLASH`.
//! 12. **Reading data is the entry point's job.** The core receives a
//!     normalised `Dataset`.
//!
//! [Vega-Lite v5]: https://vega.github.io/vega-lite/docs/
mod data;
mod error;
mod lower;
mod schema;
mod transforms;
mod types;
mod validate;

pub use data::{inline_dataset, inline_dataset_of};
pub use error::{ErrorReport, SpecError, codes};
pub use lower::{SUPPORTED_EXTENSION_MARKS, lower};
pub use schema::{schema, schema_json};
pub use types::{
    AggregateEntry, AggregateFieldSpec, AggregateSpec, BinEntry, BinParams, BinSpec, ChartonExt,
    Data, DataType, DomainSpec, Encoding, ExtensionMark, FacetDef, FieldDef, FilterEntry,
    FilterExpr, FilterValue, FoldEntry, InlineRow, InlineValue, Mark, MarkDef, MarkType,
    SPEC_SCHEMA_URL, SUPPORTED_SPEC_VERSION, ScaleDef, ScaleTypeSpec, Spec, SpecMark, StackKind,
    StackSpec, ThemeName, Transform, WindowEntry, WindowFieldSpec, WindowOpSpec,
};
pub use validate::{
    MAX_INLINE_CELLS, MAX_INLINE_ROWS, MarkRequirements, check, from_json, from_value,
    mark_requirements, validate, validate_with_data,
};
