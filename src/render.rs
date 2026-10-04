use crate::core::context::PanelContext;
use crate::visual::color::SingleColor;

/// Resolves a mark's colour from a normalized value.
///
/// `Some(f64::NAN)` means this layer maps colour but the row's value is missing
/// (e.g. a null value on a continuous colour scale); it is painted neutral grey,
/// matching ggplot2's `na.value`. `None` means the layer does not map colour at
/// all, so the mark's own fallback colour is used.
pub(crate) fn resolve_color(
    norm: Option<f64>,
    context: &PanelContext<'_>,
    fallback: &SingleColor,
) -> SingleColor {
    if let (Some(value), Some(mapping)) = (norm, &context.spec.aesthetics.color) {
        if value.is_nan() {
            return SingleColor::from_rgba(0.5, 0.5, 0.5, 1.0);
        }
        let scale = mapping.scale_impl.as_ref();
        scale
            .mapper()
            .as_ref()
            .map(|mapper| mapper.map_to_color(value, scale.logical_max()))
            .unwrap_or(*fallback)
    } else {
        *fallback
    }
}

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
