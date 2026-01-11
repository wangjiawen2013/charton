//! Charton is a powerful plotting library for Rust that provides first-class, native support
//! for Rust [Polars](https://github.com/pola-rs/polars), and offers an API similar to
//! Python's [Altair](https://altair-viz.github.io/), making it easy for users familiar with
//! declarative, instruction-based plotting to migrate. It also allows you to leverage existing
//! mature visualization ecosystems, such as Altair and [Matplotlib](https://matplotlib.org/).
//! By seamlessly integrating with [evcxr_jupyter](https://github.com/evcxr/evcxr), Charton
//! facilitates the creation of informative and aesthetically pleasing visualizations interactively,
//! making it especially well-suited for exploratory data analysis.

pub mod axis;
pub mod core;
pub mod chart;
pub mod chart2;
pub mod coord;
pub mod coordinate;
pub mod scale;
pub mod data;
pub mod encode;
pub mod error;
pub mod mark;
pub mod render;
pub mod stats;
pub mod theme;
pub mod transform;
pub mod visual;

#[cfg(not(target_arch = "wasm32"))]
pub mod bridge;

pub mod prelude {
    pub use crate::encode::{
        color::color, encoding::Encoding, opacity::opacity, shape::shape, size::size,
        stroke::stroke, stroke_width::stroke_width, text::text, theta::theta, x::x, y::y, y2::y2,
    };

    pub use crate::chart::common::{Chart, LayeredChart};
    pub use crate::coord::Scale;
    pub use crate::data::{DataFrameSource, load_dataset};

    pub use crate::transform::{
        density::{BandwidthType, DensityTransform, KernelType},
        window::{WindowFieldDef, WindowOnlyOp, WindowTransform},
    };

    pub use crate::render::line_renderer::PathInterpolation;
    pub use crate::theme::Theme;
    pub use crate::visual::color::{ColorMap, ColorPalette, SingleColor};
    pub use crate::visual::shape::PointShape;

    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::bridge::base::{Altair, Matplotlib, Plot, Visualization};
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::data; // Macro data!
}
