//! Charton is a powerful plotting library for Rust that provides first-class, native support
//! for Rust [Polars](https://github.com/pola-rs/polars), and offers an API similar to
//! Python's [Altair](https://altair-viz.github.io/), making it easy for users familiar with
//! declarative, instruction-based plotting to migrate. It also allows you to leverage existing
//! mature visualization ecosystems, such as Altair and [Matplotlib](https://matplotlib.org/).
//! By seamlessly integrating with [evcxr_jupyter](https://github.com/evcxr/evcxr), Charton
//! facilitates the creation of informative and aesthetically pleasing visualizations interactively,
//! making it especially well-suited for exploratory data analysis.

pub mod encode;
pub mod chart;
pub mod coord;
pub mod data;
pub mod transform;
pub mod stats;
pub mod error;
pub mod mark;
pub mod theme;
pub mod axis;
pub mod render;
pub mod visual;

#[cfg(not(target_arch = "wasm32"))]
pub mod bridge;

pub mod prelude {
    pub use crate::encode::{
        x::x,
        y::y,
        y2::y2,
        theta::theta,
        color::color,
        shape::shape,
        size::size,
        opacity::opacity,
        stroke::stroke,
        stroke_width::stroke_width,
        text::text,
        encoding::Encoding,
    };

    pub use crate::chart::common::{Chart, LayeredChart};
    pub use crate::data::{DataFrameSource, load_dataset};
    pub use crate::coord::Scale;

    pub use crate::transform::{
        density::{DensityTransform, BandwidthType, KernelType},
        window::{WindowTransform, WindowOnlyOp, WindowFieldDef},
    };

    pub use crate::render::line_renderer::PathInterpolation;
    pub use crate::visual::color::{SingleColor, ColorMap, ColorPalette};
    pub use crate::visual::shape::PointShape;
    pub use crate::theme::Theme;

    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::bridge::base::{Visualization, Plot, Altair, Matplotlib};
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::data; // Macro data!
}
