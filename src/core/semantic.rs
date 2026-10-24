//! The semantic structure of a rendered chart.
//!
//! A chart is not just a bag of shapes: every shape plays a role. A `<line>` may
//! be a grid line, a tick, an axis spine or a data mark, and a `<text>` may be a
//! tick label, an axis title, a legend entry or the chart title. The drawing
//! primitives ([`RenderBackend`](crate::core::layer::RenderBackend)) only see
//! coordinates and colors; this module is how the *pipeline* tells the backend
//! what it is painting.
//!
//! # Three channels of meaning
//!
//! 1. A [`Group`](crate::core::semantic::Group) is a **role scope**: everything
//!    drawn while it is open shares one [`Role`](crate::core::semantic::Role)
//!    (axis, grid, legend, …). Roles are a closed, enumerated set, so they can
//!    become CSS classes and part of the public API.
//! 2. A group also carries **identity** — which layer, which series, which mark
//!    type, which axis. Identity is open-ended (it comes from the data), so it
//!    becomes a `data-*` attribute rather than a class. Keeping data values out
//!    of selectors avoids escaping problems and protects the hot path.
//! 3. An [`Element`](crate::core::semantic::Element) annotates **one
//!    observation**: the primitives drawn between `begin_element` and
//!    `end_element` stand for a single tick label or bar. Its value is written on
//!    the primitive itself, because a rule matching a parent container cannot
//!    override the presentation attributes a child already carries.
//!
//! # What backends do with it
//!
//! | Backend | Expression |
//! |---|---|
//! | SVG | `<g class="charton-…">`, `data-*`, `aria-*`, and `data-value` on the primitive |
//! | PDF | a marked-content section per whole accessible unit, collected into a tag tree (see [`Structure`](crate::core::semantic::Structure)) |
//! | raster, GPU | only the clip rectangle: roles cost them nothing |
//!
//! Scopes are deliberately coarse. Their number is O(ticks + layers), not
//! O(points), so this stays compatible with the backends' streaming,
//! allocation-free design; only the elements a stylesheet is likely to address
//! individually (tick labels, bars) get a per-observation annotation.

use crate::coordinate::Rect;

/// Which axis of a coordinate system a group belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AxisKind {
    /// The horizontal value axis (a Cartesian bottom axis, or a geo x axis).
    X,
    /// The vertical value axis (a Cartesian left axis, or a geo y axis).
    Y,
    /// A polar radius axis.
    Radius,
    /// A polar angular axis.
    Theta,
}

impl AxisKind {
    /// The identity class for this axis, e.g. `charton-axis-x`.
    pub const fn class(self) -> &'static str {
        match self {
            Self::X => "charton-axis-x",
            Self::Y => "charton-axis-y",
            Self::Radius => "charton-axis-radius",
            Self::Theta => "charton-axis-theta",
        }
    }

    /// The matching `data-axis` value.
    pub const fn data_value(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Radius => "radius",
            Self::Theta => "theta",
        }
    }
}

/// The semantic role of a group of drawn elements.
///
/// Every variant owns exactly one CSS class (see [`Role::class`]). The set is
/// `#[non_exhaustive]`: new roles may be added, but existing names never change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Role {
    /// The canvas fill behind everything else (decorative).
    Background,
    /// The chart title.
    Title,
    /// The label inside a facet header strip.
    FacetLabel,
    /// The clipped plotting area that holds the marks.
    Plot,
    /// Grid lines (decorative).
    Grid,
    /// The frame around the plotting area (decorative).
    Frame,
    /// One coordinate axis, including its spines, ticks and title.
    Axis(AxisKind),
    /// The axis spine.
    AxisLine,
    /// Tick marks.
    Tick,
    /// Tick labels.
    TickLabel,
    /// The axis title.
    AxisTitle,
    /// Data marks of one layer.
    Mark,
    /// The marks of one series inside a layer (the group a color, shape or path
    /// group key produced).
    Series,
    /// One legend block (its title and entries, or a colour bar), carrying the
    /// field it explains and the clip that keeps it off the plot.
    Legend,
    /// The title above a legend or color bar.
    LegendTitle,
    /// One legend entry (symbol plus label).
    LegendEntry,
    /// The symbol drawn for a legend entry.
    LegendSwatch,
    /// The text of a legend entry.
    LegendLabel,
    /// A continuous color bar.
    ColorBar,
}

impl Role {
    /// The primary CSS class for this role.
    pub const fn class(self) -> &'static str {
        match self {
            Self::Background => "charton-background",
            Self::Title => "charton-title",
            Self::FacetLabel => "charton-facet-label",
            Self::Plot => "charton-plot",
            Self::Grid => "charton-grid",
            Self::Frame => "charton-frame",
            Self::Axis(_) => "charton-axis",
            Self::AxisLine => "charton-axis-line",
            Self::Tick => "charton-tick",
            Self::TickLabel => "charton-tick-label",
            Self::AxisTitle => "charton-axis-title",
            Self::Mark => "charton-mark",
            Self::Series => "charton-series",
            Self::Legend => "charton-legend",
            Self::LegendTitle => "charton-legend-title",
            Self::LegendEntry => "charton-legend-entry",
            Self::LegendSwatch => "charton-legend-swatch",
            Self::LegendLabel => "charton-legend-label",
            Self::ColorBar => "charton-colorbar",
        }
    }

    /// A second class refining the role, when the role has a closed set of
    /// variants worth targeting (currently the axis side).
    pub const fn qualifier(self) -> Option<&'static str> {
        match self {
            Self::Axis(kind) => Some(kind.class()),
            _ => None,
        }
    }

    /// Whether the group only repeats or frames information that is available
    /// elsewhere (grid, frame, background), so assistive technology can skip it.
    pub const fn is_decorative(self) -> bool {
        matches!(self, Self::Background | Self::Grid | Self::Frame)
    }
}

/// The identity of the single element drawn inside an element scope.
///
/// This is the per-element counterpart of [`Group`]: the group says *which
/// series* a run of primitives belongs to, an `Element` says *which observation*
/// one primitive stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Element<'a> {
    /// The data value the element stands for. Becomes `data-value`.
    pub value: &'a str,
    /// The series it belongs to, when the mark is drawn as a flat stream and has
    /// no [`Role::Series`] scope of its own — a bar of a grouped bar chart.
    /// Becomes `data-series`.
    pub series: Option<&'a str>,
}

impl<'a> Element<'a> {
    /// An element that stands for `value`.
    pub const fn new(value: &'a str) -> Self {
        Self {
            value,
            series: None,
        }
    }

    /// Attaches the series the element belongs to, when it is known.
    pub const fn with_series(mut self, series: Option<&'a str>) -> Self {
        self.series = series;
        self
    }
}

/// How a role maps onto the structure of a *tagged* document.
///
/// This is the backend-neutral half of PDF tagging, kept next to [`Role`] so one
/// description of the chart drives both formats: SVG expresses the same roles as
/// classes and `aria-*` attributes; a tagged PDF expresses them as structure
/// elements. Formats without a structure tree (raster, GPU) never look at it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Structure {
    /// Purely decorative content, excluded from the logical structure.
    Artifact,
    /// The chart itself: a figure, announced with the chart's title/description.
    Figure,
    /// A caption, i.e. the chart title.
    Caption,
    /// A block of text that belongs to the document, e.g. the axes or the legend.
    Paragraph,
}

impl Role {
    /// The structure element this role maps to, if it is a tag-worthy unit.
    ///
    /// Roles not listed here (plot sub-parts, tick labels, legend entries, …)
    /// belong to the enclosing unit: a tagged-document section cannot nest, so
    /// only whole accessible units start one.
    pub const fn structure(self) -> Option<Structure> {
        // Decoration is defined once, by `is_decorative`, and excluded here.
        if self.is_decorative() {
            return Some(Structure::Artifact);
        }
        match self {
            Self::Title => Some(Structure::Caption),
            Self::Plot => Some(Structure::Figure),
            Self::FacetLabel | Self::Axis(_) | Self::Legend => Some(Structure::Paragraph),
            _ => None,
        }
    }
}

/// How much accessibility metadata an exported document should carry.
///
/// This is a property of the *document*, not of the chart, so it lives in
/// `SvgOptions` / `PdfOptions` rather than in the theme. PDF has no
/// navigable-graphics mode, so it treats every enabled variant the same and only
/// distinguishes "tagged" from "plain".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum A11y {
    /// No accessibility metadata: the document describes only what it draws.
    Off,
    /// The chart is one image: `role="img"` on the root plus `<title>`/`<desc>`.
    ///
    /// This satisfies WCAG 2.1 SC 1.1.1 and is what almost every chart needs.
    /// The `role="img"` root collapses the whole subtree into a single node, so
    /// the internal groups need no `aria-hidden`.
    #[default]
    Image,
    /// The chart is a navigable graphic document (WAI-ARIA Graphics Module):
    /// the root is `graphics-document`, marks are `graphics-object` with
    /// `aria-label`s, and decorative groups are `aria-hidden`.
    ///
    /// Use this when the chart is meant to be explored element by element
    /// rather than as a single image.
    Document,
}

impl A11y {
    /// Whether accessibility attributes should be emitted at all.
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// Whether decorative groups should be hidden from assistive technology.
    ///
    /// Only meaningful in [`A11y::Document`] mode: under `role="img"` the whole
    /// subtree is already a single node.
    pub const fn hides_decorative(self) -> bool {
        matches!(self, Self::Document)
    }
}

/// A semantic scope: everything drawn between [`Group`] begin and end shares
/// one role (and optionally one identity).
///
/// Construct one with [`Group::new`] and refine it with the builders, e.g.
///
/// ```
/// use charton::core::semantic::{Group, Role};
/// let group = Group::new(Role::Mark).layer(1).mark("line");
/// assert_eq!(group.layer, Some(1));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Group<'a> {
    /// What this group is. Always required: there is no anonymous scope, since
    /// an anonymous scope could only be expressed as a bare `<g>`.
    pub role: Role,
    /// Index of the layer in the chart's layer stack, for marks.
    pub layer: Option<usize>,
    /// The data this scope represents: the series name inside a mark group, or
    /// the field a legend explains.
    pub series: Option<&'a str>,
    /// The mark type (`"line"`, `"point"`, `"bar"`, ...), for marks. It becomes
    /// the `charton-mark-<type>` class.
    pub mark: Option<&'a str>,
    /// A human-readable name for the group, used as `aria-label` in
    /// [`A11y::Document`] mode.
    pub label: Option<&'a str>,
    /// Clip everything in this group (and its children) to this rectangle.
    pub clip: Option<Rect>,
}

impl<'a> Group<'a> {
    /// A group with no identity and no clip.
    pub const fn new(role: Role) -> Self {
        Self {
            role,
            layer: None,
            series: None,
            mark: None,
            label: None,
            clip: None,
        }
    }

    /// Attaches the layer index (marks).
    pub const fn layer(mut self, index: usize) -> Self {
        self.layer = Some(index);
        self
    }

    /// Attaches the series name (marks).
    pub const fn series(mut self, series: &'a str) -> Self {
        self.series = Some(series);
        self
    }

    /// Attaches the series name when the grouping produced one.
    ///
    /// An ungrouped mark still gets a [`Role::Series`] scope, so every chart has
    /// the same shape; it simply carries no `data-series` attribute.
    pub const fn series_opt(mut self, series: Option<&'a str>) -> Self {
        self.series = series;
        self
    }

    /// Attaches the mark type (marks).
    pub const fn mark(mut self, mark: &'a str) -> Self {
        self.mark = Some(mark);
        self
    }

    /// Attaches an accessible name.
    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Clips this group to `rect`.
    pub const fn clip(mut self, rect: Rect) -> Self {
        self.clip = Some(rect);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_map_to_stable_class_names() {
        assert_eq!(Role::Title.class(), "charton-title");
        assert_eq!(Role::Axis(AxisKind::X).class(), "charton-axis");
        assert_eq!(Role::Axis(AxisKind::X).qualifier(), Some("charton-axis-x"));
        assert_eq!(Role::Legend.qualifier(), None);
        assert_eq!(Role::TickLabel.qualifier(), None);
        // Every class is namespaced, so an inlined chart cannot collide with the
        // host page's own `.axis` / `.title` rules.
        for role in [
            Role::Background,
            Role::Title,
            Role::FacetLabel,
            Role::Plot,
            Role::Grid,
            Role::Frame,
            Role::Axis(AxisKind::Y),
            Role::AxisLine,
            Role::Tick,
            Role::TickLabel,
            Role::AxisTitle,
            Role::Mark,
            Role::Series,
            Role::Legend,
            Role::LegendTitle,
            Role::LegendEntry,
            Role::LegendSwatch,
            Role::LegendLabel,
            Role::ColorBar,
        ] {
            assert!(role.class().starts_with("charton-"), "{role:?}");
            if let Some(qualifier) = role.qualifier() {
                assert!(qualifier.starts_with("charton-"), "{role:?}");
            }
        }
    }

    #[test]
    fn only_the_scaffolding_is_decorative() {
        assert!(Role::Background.is_decorative());
        assert!(Role::Grid.is_decorative());
        assert!(Role::Frame.is_decorative());
        assert!(!Role::Mark.is_decorative());
        assert!(!Role::TickLabel.is_decorative());
        assert!(!Role::Axis(AxisKind::X).is_decorative());
    }

    #[test]
    fn structure_maps_only_whole_units() {
        // Decorative scaffolding is excluded from the logical structure.
        assert_eq!(Role::Grid.structure(), Some(Structure::Artifact));
        assert_eq!(Role::Frame.structure(), Some(Structure::Artifact));
        assert_eq!(Role::Background.structure(), Some(Structure::Artifact));
        // The chart body is the figure; its parts belong to it.
        assert_eq!(Role::Plot.structure(), Some(Structure::Figure));
        assert_eq!(Role::Title.structure(), Some(Structure::Caption));
        assert_eq!(
            Role::Axis(AxisKind::X).structure(),
            Some(Structure::Paragraph)
        );
        assert_eq!(Role::Legend.structure(), Some(Structure::Paragraph));
        assert_eq!(Role::FacetLabel.structure(), Some(Structure::Paragraph));
        // Everything that is part of a unit belongs to the enclosing tag.
        for role in [
            Role::Mark,
            Role::Series,
            Role::Tick,
            Role::TickLabel,
            Role::AxisLine,
            Role::AxisTitle,
            Role::LegendEntry,
            Role::LegendSwatch,
            Role::LegendLabel,
            Role::ColorBar,
        ] {
            assert_eq!(role.structure(), None, "{role:?}");
        }
    }

    #[test]
    fn a11y_modes_are_ordered_by_how_much_they_emit() {
        assert!(!A11y::Off.is_enabled());
        assert!(A11y::Image.is_enabled());
        assert!(A11y::Document.is_enabled());
        // `role="img"` already collapses the subtree, so hiding children is
        // only needed (and only emitted) in document mode.
        assert!(!A11y::Image.hides_decorative());
        assert!(A11y::Document.hides_decorative());
    }

    #[test]
    fn element_identity_is_optional_about_the_series() {
        let element = Element::new("2020");
        assert_eq!(element.value, "2020");
        assert_eq!(element.series, None);
        assert_eq!(
            Element::new("2020").with_series(Some("A")).series,
            Some("A")
        );
    }

    #[test]
    fn group_builders_are_chainable() {
        let clip = Rect::new(1.0, 2.0, 3.0, 4.0);
        let group = Group::new(Role::Mark)
            .layer(2)
            .series("Ford")
            .mark("line")
            .label("Ford")
            .clip(clip);
        assert_eq!(group.role, Role::Mark);
        assert_eq!(group.layer, Some(2));
        assert_eq!(group.series, Some("Ford"));
        assert_eq!(group.mark, Some("line"));
        assert_eq!(group.label, Some("Ford"));
        let clipped = group.clip.expect("clip is recorded");
        assert_eq!(
            (clipped.x, clipped.y, clipped.width, clipped.height),
            (1.0, 2.0, 3.0, 4.0)
        );
    }
}
