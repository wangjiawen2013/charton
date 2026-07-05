use crate::core::composite::LayeredChart;
use crate::core::conversion::IntoLayered;
use crate::core::layer::{RenderBackend, TextConfig};
use crate::error::ChartonError;
use crate::render::backend::raster::RasterBackend;
use crate::render::backend::wgpu::WgpuBackend;
use std::sync::mpsc;

/// A standalone GPU renderer that internally manages its own WGPU instance.
///
/// This module provides a high-level API designed to simplify user operations.
/// It encapsulates the entire WGPU lifecycle (device/queue creation) and performs
/// offscreen rendering, returning a simple pixel buffer (`Vec<u8>`).
///
/// # Performance Considerations (Important!)
/// `WgpuRenderer` is optimized for **ease of use and one-shot generation**
/// (e.g., CLI tools, exporting images, server-side rendering).
///
/// **Do not use this in a high-FPS interactive game loop** (like `RedrawRequested` in winit/Bevy).
/// Calling `.render()` performs full GPU-to-CPU memory readbacks, allocates new textures,
/// and blocks the thread on every call.
///
/// For interactive GUI applications, inject your app's existing WGPU context directly
/// into the chart using `chart.render_to_surface(&mut app_backend, &app_view)`.
/// This keeps all rendering strictly on the GPU (zero-copy).
///
/// # Example
/// ```ignore
/// // Ideal for exporting static images
/// let mut renderer = WgpuRenderer::new();
/// let pixels = renderer.render(&chart, 512, 512, 1.0)?;
/// std::fs::write("output.rgba", pixels);
/// ```
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl WgpuRenderer {
    /// Creates a new renderer, automatically initializing the WGPU instance, adapter, and device.
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            display: None,
        });

        // Request a high-performance GPU adapter synchronously
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("charton: no suitable GPU adapter found");

        // Request the logical device and command queue
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("charton WgpuRenderer"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .expect("charton: failed to create wgpu device");

        Self { device, queue }
    }

    /// Renders the chart and returns the raw RGBA pixel data.
    ///
    /// `scale_factor` is used for DPI scaling; typically pass `1.0` for standard resolution.
    pub fn render<C>(
        &mut self,
        chart: &C,
        width: u32,
        height: u32,
        scale_factor: f32,
    ) -> Result<Vec<u8>, ChartonError>
    where
        C: IntoLayered + Clone,
    {
        let layered: LayeredChart = chart.clone().into();

        // 1. Create an offscreen render target texture
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("charton render target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            // Requires RENDER_ATTACHMENT for drawing and COPY_SRC to read back to CPU
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 2. Render geometry on the GPU and collect the deferred text ledger
        let text_ledger =
            pollster::block_on(self.render_geometry(&layered, &view, width, height, scale_factor))?;

        // 3. Read back the pixel data from GPU to CPU memory
        let mut pixels = self.readback_pixels(&texture, width, height)?;

        // 4. Composite text using CPU rasterizer if any text exists in the ledger
        if !text_ledger.is_empty() {
            self.composite_text(&mut pixels, width, height, scale_factor, text_ledger)?;
        }

        Ok(pixels)
    }

    // ── Internal Methods ──────────────────────────────────────

    /// Internally bridges to `render_to_surface`, generating the GPU primitives.
    async fn render_geometry(
        &mut self,
        chart: &LayeredChart,
        view: &wgpu::TextureView,
        width: u32,
        height: u32,
        scale_factor: f32,
    ) -> Result<Vec<TextConfig>, ChartonError> {
        let mut backend = WgpuBackend::new(
            self.device.clone(),
            self.queue.clone(),
            width,
            height,
            scale_factor,
        )
        .await;

        // Leverages the shared rendering path to output to our internal texture view
        chart.render_to_surface(&mut backend, view).await
    }

    /// Handles the async buffer mapping required to pull texture data off the GPU.
    fn readback_pixels(
        &self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, ChartonError> {
        let bytes_per_pixel = 4usize;
        let unpadded_bytes = (width as usize) * bytes_per_pixel;
        // WGPU requires buffer rows to be aligned to 256 bytes (COPY_BYTES_PER_ROW_ALIGNMENT)
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes = ((unpadded_bytes + align - 1) / align) * align;
        let buffer_size = (padded_bytes * height as usize) as u64;

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("charton readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy the texture into the readable buffer
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes as u32),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map the buffer asynchronously and wait for completion
        let slice = buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });

        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        rx.recv()
            .map_err(|_| ChartonError::Render("readback channel closed".into()))?
            .map_err(|e| ChartonError::Render(format!("buffer map failed: {e:?}")))?;

        // Strip the WGPU alignment padding to return clean contiguous RGBA data
        let data = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity(unpadded_bytes * height as usize);

        for row in 0..height as usize {
            let start = row * padded_bytes;
            pixels.extend_from_slice(&data[start..start + unpadded_bytes]);
        }

        drop(data);
        buffer.unmap();
        Ok(pixels)
    }

    /// Composites deferred text rendering operations onto the pixel buffer using CPU rasterization.
    fn composite_text(
        &self,
        rgba: &mut [u8],
        width: u32,
        height: u32,
        scale_factor: f32,
        ledger: Vec<TextConfig>,
    ) -> Result<(), ChartonError> {
        // Wrap the raw pixel buffer in a tiny_skia Pixmap for CPU drawing
        let mut pixmap = tiny_skia::Pixmap::from_vec(
            rgba.to_vec(),
            tiny_skia::IntSize::from_wh(width, height)
                .ok_or_else(|| ChartonError::Render("invalid pixmap size".into()))?,
        )
        .ok_or_else(|| ChartonError::Render("failed to create pixmap".into()))?;

        let mut backend = RasterBackend::new(&mut pixmap, scale_factor);
        for config in ledger {
            backend.draw_text(config);
        }

        // Write the composited result back to the original slice
        rgba.copy_from_slice(pixmap.data());
        Ok(())
    }
}

impl Default for WgpuRenderer {
    fn default() -> Self {
        Self::new()
    }
}
