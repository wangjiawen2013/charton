//! `charton plot` — a few flags into a chart.
//!
//! The flags become a spec, and the spec is rendered by the same path `render`
//! uses. Nothing here touches the chart builder directly: one meaning, one build
//! path, and `--emit-spec` is exactly what was drawn.

use charton::spec::{
    ChartonExt, Encoding, ExtensionMark, FieldDef, Mark, MarkType, SPEC_SCHEMA_URL, Spec, SpecMark,
};

use crate::cli::PlotArgs;
use crate::commands;
use crate::errors::CliError;
use crate::input::{self, Loaded};
use crate::output;

/// Run `charton plot`.
///
/// # Errors
///
/// Fails on unreadable data (exit 3), an invalid combination of flags (exit 4),
/// or a failed render (exit 5).
pub fn run(args: PlotArgs) -> Result<(), CliError> {
    let loaded = input::load(args.input.input.as_deref())?;
    let spec = spec_from(&args, &loaded)?;

    // Written before rendering: a document you can keep is worth more than a
    // half-finished render, and it is the same bytes either way.
    if let Some(path) = args.emit_spec.as_deref() {
        let text = commands::canonical(&spec)?;
        output::write_text(&text, Some(path), args.output.force)?;
    }

    let chart = commands::build_chart(&spec, loaded.dataset)?;
    output::write_chart(&chart, args.output.output.as_deref(), args.output.force)?;
    Ok(())
}

/// The spec these flags describe.
fn spec_from(args: &PlotArgs, loaded: &Loaded) -> Result<Spec, CliError> {
    let mark = resolve_mark(&args.mark)?;

    let encoding = Encoding {
        x: args.x.as_deref().map(FieldDef::new),
        y: args.y.as_deref().map(FieldDef::new),
        y2: args.y2.as_deref().map(FieldDef::new),
        color: args.color.as_deref().map(FieldDef::new),
        size: args.size.as_deref().map(FieldDef::new),
        shape: args.shape.as_deref().map(FieldDef::new),
        text: args.text.as_deref().map(FieldDef::new),
        path_group: args.path_group.as_deref().map(FieldDef::new),
    };

    let (vega, extension) = match mark {
        SpecMark::Vega(mark) => (Some(mark), None),
        SpecMark::Extension(mark) => (None, Some(mark)),
    };

    Ok(Spec {
        schema: Some(SPEC_SCHEMA_URL.to_owned()),
        data: Some(loaded.data.clone()),
        mark: vega.map(Mark::Type),
        encoding,
        layer: Vec::new(),
        facet: None,
        transform: Vec::new(),
        charton: extension.map(|mark| ChartonExt {
            mark: Some(mark),
            z: args.z.clone(),
        }),
        title: args.title.clone(),
        description: args.description.clone(),
        theme: None,
    })
}

/// Map a mark name to the namespace it belongs to.
///
/// The two namespaces stay separate even here: a Charton-only name never becomes
/// a Vega-Lite mark, it becomes `charton.mark`.
fn resolve_mark(name: &str) -> Result<SpecMark, CliError> {
    if let Some(mark) = MarkType::from_wire(name) {
        return Ok(SpecMark::Vega(mark));
    }
    ExtensionMark::ALL
        .iter()
        .copied()
        .find(|mark| mark.as_str() == name)
        .map(SpecMark::Extension)
        .ok_or_else(|| CliError::usage(format!("unknown mark `{name}`")))
}
