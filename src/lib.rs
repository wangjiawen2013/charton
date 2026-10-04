//! Charton is a powerful plotting library for Rust that provides first-class, native support
//! for Rust [Polars](https://github.com/pola-rs/polars), and offers an API similar to
//! Python's [Altair](https://altair-viz.github.io/), making it easy for users familiar with
//! declarative, instruction-based plotting to migrate. It also allows you to leverage existing
//! mature visualization ecosystems, such as Altair and [Matplotlib](https://matplotlib.org/).
//! By seamlessly integrating with [evcxr_jupyter](https://github.com/evcxr/evcxr), Charton
//! facilitates the creation of informative and aesthetically pleasing visualizations interactively,
//! making it especially well-suited for exploratory data analysis.
//!
//! # How a chart is built
//!
//! Every chart is a small pipeline of independent steps:
//!
//! * a **transform** summarises the data — `transform_density` estimates a
//!   smooth curve, `transform_contour` extracts iso-lines, `transform_band`
//!   turns a curve into a placed polygon;
//! * a **position** decides where marks sit when several share a category
//!   (`Position::dodge`);
//! * a **mark** draws a shape (`mark_polygon`, `mark_line`, `mark_point`, ...).
//!
//! Because the steps are independent they can be recombined. A violin is a
//! density transform plus a band plus a polygon; a contour plot is a density
//! grid plus an iso-line transform plus an open path. The convenience marks
//! (`mark_violin`, `mark_density`, `mark_contour`, `mark_density_2d`) are just
//! names for those recipes: each expands back into these same steps, so the
//! composition is always available. See `docs/src/concepts/grammar_pipeline.md`
//! for the full model.

#![warn(clippy::missing_const_for_fn)]

pub mod chart;
pub mod coordinate;
pub mod core;
pub mod datasets;
pub mod encode;
pub mod error;
pub mod facets;
pub mod mark;
pub mod position;
pub mod render;
pub mod scale;
pub mod stats;
pub mod theme;
pub mod transform;
pub mod visual;

/// Minimal re-exports of the Apache Arrow building blocks used by the
/// `arrow` feature.
///
/// Charton depends only on the lightweight `arrow-array`, `arrow-schema` and
/// `arrow-select` crates instead of the full `arrow` umbrella crate. This keeps
/// the optional Arrow integration much cheaper to compile while still exposing
/// the same type paths (`charton::arrow::array::*`, `charton::arrow::datatypes::*`, ...).
#[cfg(feature = "arrow")]
pub mod arrow {
    pub use arrow_array;
    pub use arrow_schema;
    pub use arrow_select;

    /// Arrow array types (`Array`, `Float64Array`, `RecordBatch`, ...).
    pub mod array {
        pub use arrow_array::cast::*;
        pub use arrow_array::*;
    }

    /// Arrow logical data types and primitive type aliases
    /// (`DataType`, `TimeUnit`, `Int64Type`, ...).
    pub mod datatypes {
        pub use arrow_array::types::*;
        pub use arrow_schema::*;
    }

    /// Record batch support.
    pub mod record_batch {
        pub use arrow_array::RecordBatch;
    }

    /// Compute kernels required for ingestion.
    pub mod compute {
        pub use arrow_select::concat::concat;
    }
}

/// Global macros providing syntactic sugar for data construction,
/// external library integration, and developer convenience.
#[macro_use]
pub mod macros;

pub mod alt {
    pub use crate::encode::color::color;
    pub use crate::encode::path_group::path_group;
    pub use crate::encode::shape::shape;
    pub use crate::encode::size::size;
    pub use crate::encode::text::text;
    pub use crate::encode::x::x;
    pub use crate::encode::y::y;
    pub use crate::encode::y2::y2;
}

pub mod prelude {
    pub use crate::alt;
    pub use crate::chart::Chart;
    pub use crate::coordinate::CoordSystem;
    pub use crate::coordinate::geo::GeoProjection;
    pub use crate::core::composite::LayeredChart;
    pub use crate::core::conversion::IntoLayered;
    pub use crate::core::data::{ColumnVector, Dataset, IntoColumn, ToDataset};
    pub use crate::core::guide::LegendPosition;
    pub use crate::datasets::load_dataset;
    pub use crate::facets::FacetSpec;
    pub use crate::mark::{
        area::MarkArea, bar::MarkBar, boxplot::MarkBoxplot, contour::MarkContour,
        density::MarkDensity, density_2d::MarkDensity2D, errorbar::MarkErrorBar,
        geo_path::MarkGeoPath, line::MarkLine, point::MarkPoint, rect::MarkRect, rule::MarkRule,
        text::MarkText, tick::MarkTick, violin::MarkViolin,
    };
    pub use crate::position::Position;
    pub use crate::render::line_renderer::PathInterpolation;
    pub use crate::scale::formatter::{Abbreviation, LabelFormat};
    pub use crate::scale::{Expansion, Scale};
    pub use crate::theme::{Theme, ThemeMode, TitleAnchor, TitleFrame};
    pub use crate::transform::{
        band_transform::{BandScale, BandTransform},
        box_transform::QuantileBoxTransform,
        contour_transform::{ContourLevels, ContourTransform},
        density_2d_transform::Density2DTransform,
        density_transform::{BandwidthType, DensityTransform, KernelType},
        window_transform::{WindowFieldDef, WindowOnlyOp, WindowTransform},
    };
    pub use crate::visual::color::{ColorMap, ColorPalette, SingleColor};
    pub use crate::visual::shape::PointShape;
    pub use crate::{chart, load_polars_df, load_polars_v44_52};
    pub use time as ctime;

    #[cfg(feature = "geo")]
    pub use crate::core::utils::geojson_to_dataset;
}

/// Temporary column name used internally by Polars to avoid naming conflicts.
pub(crate) const TEMP_SUFFIX: &str = "__charton_temp_n9jh3z8";

/// Represents the floating-point precision used specifically for the rendering stage.
///
/// While data processing and coordinate transformations should be performed in `f64`
/// to maintain computational accuracy and prevent rounding errors, we convert to
/// `Precision` (f32) during the final draw calls for the following reasons:
///
/// 1. **GPU Hardware Native**: Modern Graphics APIs (WGPU, Metal) are optimized for `f32`.
///    Using `f32` for rendering structures allows direct GPU memory mapping.
///
/// 2. **Memory Efficiency**: Halves the memory footprint for large point sets (e.g., in
///    scatter plots) when passing data to the rendering backends.
///
/// 3. **SVG Size Reduction**: `f32` provides sufficient precision for screen-space
///    while keeping the generated XML string lengths shorter.
pub type Precision = f32;
