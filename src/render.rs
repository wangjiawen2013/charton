pub(crate) mod area_renderer;
pub(crate) mod backend;
pub(crate) mod bar_renderer;
pub(crate) mod box_renderer;
pub(crate) mod cartesian2d_axis_renderer;
pub(crate) mod errorbar_renderer;
pub(crate) mod geo_axis_renderer;
pub(crate) mod geo_renderer;
pub(crate) mod hist_renderer;
pub(crate) mod legend_renderer;
pub(crate) mod line_renderer;
pub(crate) mod point_renderer;
pub(crate) mod polar_axis_renderer;
pub(crate) mod rect_renderer;
pub(crate) mod rule_renderer;
pub(crate) mod text_renderer;
pub(crate) mod tick_renderer;
pub mod wgpu_renderer;

// Re-export the wgpubackend and rasterbackend so `render_to_surface` can be used from extern
#[cfg(feature = "wgpu")]
pub use backend::wgpu::WgpuBackend;

#[cfg(all(feature = "wgpu", feature = "png"))]
pub use wgpu_renderer::WgpuRenderer;

#[cfg(feature = "png")]
pub use backend::raster::RasterBackend;
