//! What a mark needs from its encoding, and what each channel can carry.
//!
//! The capability table is the single source of truth for validation and for
//! `list_marks` in the MCP server. A feature is either supported on a channel or
//! it is an error; nothing is silently dropped.
use crate::spec::error::{SpecError, codes};
use crate::spec::types::BinSpec;
use crate::spec::types::{
    DataType, DomainSpec, Encoding, ExtensionMark, FieldDef, MarkType, ScaleTypeSpec, SpecMark,
    StackKind, StackSpec,
};

/// What one channel can carry.
///
/// This table is the single source of truth for both validation and lowering:
/// a feature is either supported on a channel or it is an error, never silently
/// dropped. Without it, `"aggregate"` on `x` would be accepted by the schema and
/// then quietly ignored by the builder.
#[derive(Debug, Clone, Copy)]
pub(super) struct ChannelCaps {
    aggregate: bool,
    bin: bool,
    stack: bool,
    scale: bool,
    title: bool,
}

/// Channels in the order help text lists them.
const CHANNELS: &[&str] = &[
    "x",
    "y",
    "y2",
    "color",
    "size",
    "shape",
    "text",
    "pathGroup",
];

pub(super) fn channel_caps(channel: &str) -> ChannelCaps {
    match channel {
        "x" => ChannelCaps {
            aggregate: false,
            bin: true,
            stack: false,
            scale: true,
            title: true,
        },
        "y" => ChannelCaps {
            aggregate: true,
            bin: true,
            stack: true,
            scale: true,
            title: true,
        },
        "y2" => ChannelCaps {
            aggregate: false,
            bin: false,
            stack: false,
            scale: false,
            title: false,
        },
        "color" => ChannelCaps {
            aggregate: true,
            bin: false,
            stack: false,
            scale: true,
            title: true,
        },
        "size" | "shape" => ChannelCaps {
            aggregate: false,
            bin: false,
            stack: false,
            scale: true,
            title: true,
        },
        "text" => ChannelCaps {
            aggregate: false,
            bin: false,
            stack: false,
            scale: false,
            title: false,
        },
        "pathGroup" => ChannelCaps {
            aggregate: false,
            bin: false,
            stack: false,
            scale: true,
            title: false,
        },
        _ => ChannelCaps {
            aggregate: false,
            bin: false,
            stack: false,
            scale: false,
            title: false,
        },
    }
}

/// Whether this mark bins this channel.
///
/// From the core: `rect` bins both axes (`resolve_pre_transform_encodings`),
/// `hist` bins x. Every other mark reads exact values, so a `bin` there would be
/// a request Charton cannot honour.
pub(super) fn mark_bins_channel(mark: SpecMark, channel: &str) -> bool {
    matches!(
        (mark, channel),
        (SpecMark::Vega(MarkType::Rect), "x" | "y")
            | (SpecMark::Extension(ExtensionMark::Hist), "x")
    )
}

/// Which channels support `feature`, for a hint that names the alternative.
pub(super) fn channels_supporting(feature: &str) -> Vec<&'static str> {
    CHANNELS
        .iter()
        .copied()
        .filter(|channel| {
            let caps = channel_caps(channel);
            match feature {
                "aggregate" => caps.aggregate,
                "bin" => caps.bin,
                "stack" => caps.stack,
                "scale" => caps.scale,
                "title" => caps.title,
                _ => false,
            }
        })
        .collect()
}

/// The fix for a feature-placement error, worded for the feature at hand.
pub(super) fn feature_hint(feature: &str, mark: SpecMark) -> String {
    if feature == "bin" {
        return format!(
            "Only `rect` (on x and y) and `charton.mark: hist` (on x) bin their input; `{}` reads exact values. Bin upstream instead.",
            mark.as_str()
        );
    }

    let elsewhere = channels_supporting(feature);
    if elsewhere.is_empty() {
        return format!("`{feature}` is not supported on any channel.");
    }
    format!(
        "Supported on: {}. Move it there, or drop it.",
        elsewhere
            .iter()
            .map(|channel| format!("`{channel}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(super) fn check_channels(mark: SpecMark, encoding: &Encoding, errors: &mut Vec<SpecError>) {
    let (required, one_of) = channel_rules(mark);
    for channel in required {
        if !has_channel(encoding, channel) {
            errors.push(
                SpecError::new(
                    codes::MISSING_FIELD,
                    "/encoding",
                    format!(
                        "{} needs {} `{channel}` channel",
                        mark.as_str(),
                        article(channel)
                    ),
                )
                .with_hint(format!(
                    "Add `\"{channel}\": {{ \"field\": \"...\" }}` to `encoding`."
                )),
            );
        }
    }
    if !one_of.is_empty() && !one_of.iter().any(|channel| has_channel(encoding, channel)) {
        errors.push(
            SpecError::new(
                codes::MISSING_FIELD,
                "/encoding",
                format!(
                    "{} needs at least one of {}",
                    mark.as_str(),
                    one_of
                        .iter()
                        .map(|channel| format!("`{channel}`"))
                        .collect::<Vec<_>>()
                        .join(" or ")
                ),
            )
            .with_hint("Add one of them to `encoding`."),
        );
    }

    for channel in forbidden_channels(mark) {
        if has_channel(encoding, channel) {
            errors.push(
                SpecError::new(
                    codes::MARK_CHANNEL,
                    format!("/encoding/{channel}"),
                    format!("{} does not render `{channel}`", mark.as_str()),
                )
                .with_hint(format!(
                    "Remove `{channel}`, or use a mark that supports it."
                )),
            );
        }
    }

    for (channel, def) in encoding.present() {
        let caps = channel_caps(channel);
        let features = [
            ("aggregate", def.aggregate.is_some(), caps.aggregate),
            ("bin", def.bin.is_some(), mark_bins_channel(mark, channel)),
            ("stack", def.stack.is_some(), caps.stack),
            ("scale", def.scale.is_some(), caps.scale),
            ("title", def.title.is_some(), caps.title),
        ];
        for (feature, used, supported) in features {
            if !used || supported {
                continue;
            }
            errors.push(
                SpecError::new(
                    codes::UNSUPPORTED_FIELD,
                    format!("/encoding/{channel}/{feature}"),
                    format!("`{feature}` is not supported on `{channel}`"),
                )
                .with_hint(feature_hint(feature, mark)),
            );
        }

        // `bin: false` and `bin.step` cannot be honoured: the marks that bin
        // pick their own resolution, and a silent no-op would contradict the
        // document. Rejecting is the only honest answer.
        match def.bin {
            Some(BinSpec::Enabled(false)) => errors.push(
                SpecError::new(
                    codes::UNSUPPORTED_FIELD,
                    format!("/encoding/{channel}/bin"),
                    "Binning cannot be switched off: the mark decides it",
                )
                .with_hint("Omit `bin`; binned marks bin, others do not bin at all."),
            ),
            Some(BinSpec::Params(params)) if params.step.is_some() => errors.push(
                SpecError::new(
                    codes::UNSUPPORTED_FIELD,
                    format!("/encoding/{channel}/bin/step"),
                    "A fixed bin `step` is not supported",
                )
                .with_hint("Use `maxbins`, and let the mark choose the width."),
            ),
            _ => {}
        }

        if def.aggregate.is_some() && def.bin.is_some() {
            errors.push(
                SpecError::new(
                    codes::UNSUPPORTED_FIELD,
                    format!("/encoding/{channel}"),
                    "`aggregate` and `bin` on the same channel are not supported",
                )
                .with_hint("Aggregate first, then bin the result, in two steps."),
            );
        }
    }

    check_domains(encoding, errors);

    if let Some(StackSpec::Mode(StackKind::None)) = encoding.y.as_ref().and_then(|def| def.stack)
        && matches!(mark, SpecMark::Extension(ExtensionMark::Violin))
    {
        errors.push(
            SpecError::new(
                codes::MARK_CHANNEL,
                "/encoding/y/stack",
                "`violin` is always stacked along its own density axis",
            )
            .with_hint("Remove `stack`."),
        );
    }
}

/// A domain must agree with the scale that reads it.
pub(super) fn check_domains(encoding: &Encoding, errors: &mut Vec<SpecError>) {
    for (channel, def) in encoding.present() {
        let Some(scale) = def.scale.as_ref() else {
            continue;
        };
        let Some(domain) = scale.domain.as_ref() else {
            continue;
        };
        let kind = effective_scale(def);
        let at = format!("/encoding/{channel}/scale/domain");

        match (domain, kind) {
            (DomainSpec::Continuous(_), Some(ScaleTypeSpec::Time | ScaleTypeSpec::Utc)) => errors
                .push(
                    SpecError::new(
                        codes::UNSUPPORTED_FIELD,
                        at,
                        "Numeric domains on a time scale are not supported",
                    )
                    .with_hint("Leave the domain out; time domains are inferred from the data."),
                ),
            (
                DomainSpec::Continuous(_),
                Some(ScaleTypeSpec::Band | ScaleTypeSpec::Point | ScaleTypeSpec::Ordinal),
            ) => {
                errors.push(
                    SpecError::new(
                        codes::TYPE_MISMATCH,
                        at,
                        "A two-number domain cannot describe a categorical scale",
                    )
                    .with_hint("Use a list of category names instead."),
                );
            }
            (DomainSpec::Discrete(_), Some(ScaleTypeSpec::Linear | ScaleTypeSpec::Log)) => {
                errors.push(
                    SpecError::new(
                        codes::TYPE_MISMATCH,
                        at,
                        "A list of categories cannot describe a continuous scale",
                    )
                    .with_hint("Use `[min, max]`, or a categorical scale type."),
                );
            }
            _ => {}
        }
    }
}

/// `a` or `an`, for messages that are read by people.
pub(super) const fn article(word: &str) -> &'static str {
    match word.as_bytes().first() {
        Some(b'a' | b'e' | b'i' | b'o' | b'u') => "an",
        _ => "a",
    }
}

/// What a mark needs from its encoding.
///
/// The same rules validation enforces, exposed so a tool can tell an author (or
/// an agent) what a mark expects without re-deriving it. One source of truth:
/// `list_marks` in the MCP server reads this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkRequirements {
    /// Channels that must all be present.
    pub required: Vec<&'static str>,
    /// Channels of which at least one must be present.
    pub one_of: Vec<&'static str>,
    /// Channels the mark cannot render.
    pub forbidden: Vec<&'static str>,
}

/// The channel rules for a mark.
#[must_use]
pub fn mark_requirements(mark: SpecMark) -> MarkRequirements {
    let (required, one_of) = channel_rules(mark);
    MarkRequirements {
        required,
        one_of,
        forbidden: forbidden_channels(mark),
    }
}

/// Required channels, and channels that must not be present.
///
/// Derived from what the core transforms actually require, not from theory: a
/// rule the renderer does not enforce would reject working charts.
pub(super) fn channel_rules(mark: SpecMark) -> (Vec<&'static str>, Vec<&'static str>) {
    use ExtensionMark as E;
    use MarkType as V;

    // (all of these are required, at least one of those is required).
    match mark {
        SpecMark::Vega(V::Point) => (vec!["x"], vec![]),
        // A rule spans the plot, so the core needs both positions even though
        // Vega-Lite accepts a horizontal rule with only `y`.
        SpecMark::Vega(
            V::Line | V::Bar | V::Area | V::Tick | V::Boxplot | V::Errorbar | V::Rule,
        ) => (vec!["x", "y"], vec![]),
        SpecMark::Vega(V::Text) => (vec!["x", "y", "text"], vec![]),
        // A heatmap cell is positioned by two axes and carries its value in
        // colour; the core refuses the mark without all three.
        SpecMark::Vega(V::Rect) => (vec!["x", "y", "color"], vec![]),
        // The other position is generated by the transform.
        SpecMark::Extension(E::Hist | E::Density) => (vec!["x"], vec![]),
        SpecMark::Extension(E::Violin | E::QuantileBox) => (vec![], vec!["x", "y"]),
        SpecMark::Extension(E::Density2d | E::Contour | E::Beeswarm | E::Polygon) => {
            (vec!["x", "y"], vec![])
        }
        SpecMark::Extension(E::Path) => (vec!["x", "y", "pathGroup"], vec![]),
    }
}

/// Channels a mark cannot render, however plausible they look.
///
/// `text` on a point draws nothing; `shape` only means something for symbols;
/// `y2` is a span only area, bar, rule and errorbar can draw; `pathGroup` is how
/// a line knows which rows form one path.
pub(super) fn forbidden_channels(mark: SpecMark) -> Vec<&'static str> {
    use ExtensionMark as E;
    use MarkType as V;

    let mut forbidden = Vec::new();
    if !matches!(mark, SpecMark::Vega(V::Text)) {
        forbidden.push("text");
    }
    if !matches!(mark, SpecMark::Vega(V::Point)) {
        forbidden.push("shape");
    }
    if !matches!(
        mark,
        SpecMark::Vega(V::Area | V::Bar | V::Rule | V::Errorbar)
    ) {
        forbidden.push("y2");
    }
    // A line needs to know which rows form one path; a polygon needs the same
    // to draw more than one shape.
    if !matches!(
        mark,
        SpecMark::Vega(V::Line) | SpecMark::Extension(E::Path | E::Polygon)
    ) {
        forbidden.push("pathGroup");
    }
    forbidden
}

pub(super) fn has_channel(encoding: &Encoding, channel: &str) -> bool {
    encoding.present().any(|(name, _)| name == channel)
}

/// The scale a channel will use: an explicit `scale.type` wins, then a declared
/// field `type`, otherwise the data decides.
pub(super) fn effective_scale(def: &FieldDef) -> Option<ScaleTypeSpec> {
    def.scale
        .as_ref()
        .and_then(|scale| scale.scale_type)
        .or_else(|| {
            def.data_type.map(|data_type| match data_type {
                DataType::Quantitative => ScaleTypeSpec::Linear,
                DataType::Nominal | DataType::Ordinal => ScaleTypeSpec::Band,
                DataType::Temporal => ScaleTypeSpec::Time,
            })
        })
}

// ---------------------------------------------------------------------------
// Stage 3: semantics that need the data
// ---------------------------------------------------------------------------
