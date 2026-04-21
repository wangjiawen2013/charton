//! Charton is a powerful plotting library for Rust that provides first-class, native support
//! for Rust [Polars](https://github.com/pola-rs/polars), and offers an API similar to
//! Python's [Altair](https://altair-viz.github.io/), making it easy for users familiar with
//! declarative, instruction-based plotting to migrate. It also allows you to leverage existing
//! mature visualization ecosystems, such as Altair and [Matplotlib](https://matplotlib.org/).
//! By seamlessly integrating with [evcxr_jupyter](https://github.com/evcxr/evcxr), Charton
//! facilitates the creation of informative and aesthetically pleasing visualizations interactively,
//! making it especially well-suited for exploratory data analysis.

pub mod chart;
pub mod coordinate;
pub mod core;
pub mod datasets;
pub mod encode;
pub mod error;
pub mod facets;
pub mod mark;
pub mod render;
pub mod scale;
pub mod stats;
pub mod theme;
pub mod transform;
pub mod visual;

pub mod bridge;

#[cfg(feature = "arrow")]
pub use arrow;

#[cfg(feature = "python")]
pub use crate::data; // Macro data!

/// Global macros providing syntactic sugar for data construction,
/// external library integration, and developer convenience.
#[macro_use]
pub mod macros;

pub mod alt {
    pub use crate::encode::color::color;
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
    pub use crate::core::composite::LayeredChart;
    pub use crate::core::conversion::IntoLayered;
    pub use crate::core::data::{ColumnVector, Dataset, IntoColumn, ToDataset};
    pub use crate::datasets::load_dataset;
    pub use crate::render::line_renderer::PathInterpolation;
    pub use crate::scale::{Expansion, Scale};
    pub use crate::theme::Theme;
    pub use crate::transform::{
        density_transform::{BandwidthType, DensityTransform, KernelType},
        window_transform::{WindowFieldDef, WindowOnlyOp, WindowTransform},
    };
    pub use crate::visual::color::{ColorMap, ColorPalette, SingleColor};
    pub use crate::visual::shape::PointShape;
    pub use crate::{chart, load_polars_df, load_polars_v42_52};
    pub use time::OffsetDateTime;
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
