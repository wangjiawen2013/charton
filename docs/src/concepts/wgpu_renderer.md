# The WgpuRenderer Internals

While Charton's declarative API makes building charts simple, the engine that translates those declarations into pixels is highly complex. The `WgpuRenderer` is a standalone, headless GPU rendering backend designed to encapsulate the entire WGPU lifecycle.

Understanding how `WgpuRenderer` works under the hood will give you a profound appreciation for Charton's architectural trade-offs, particularly regarding memory alignment, GPU-to-CPU data transfers, and text rendering.

## The Golden Rule: Use Cases & Anti-Patterns

Before diving into the mechanics, it is crucial to understand the intended use case of `WgpuRenderer`.

### When to Use It (The Black-Box Paradigm)

`WgpuRenderer` is optimized for ease of use and one-shot generation. It acts as a black box: data goes in, and a raw `Vec<u8>` pixel buffer comes out. It is ideal for:

* CLI Tools & Scripts: Generating charts in terminal applications.
* Server-Side Rendering (SSR): Generating static images on a backend server to serve over HTTP.
* Static Image Exports: Saving charts as .png or .svg files to disk.
* Decoupled GUIs: Rendering charts in desktop applications (like winit) where isolating WGPU dependencies is more critical than achieving high framerates.

### When NOT to Use It (The Anti-Pattern)

Do not use this in a high-FPS interactive game loop (e.g., inside Bevy systems or egui's immediate mode UI running at 60 FPS).

Every time you call `renderer.render()`, the engine performs a full GPU-to-CPU memory readback, allocates new textures, and heavily blocks the thread. For interactive GUI applications, you should inject your app's existing WGPU context directly into the chart using `chart.render_to_surface(&mut app_backend, &app_view)` to achieve Zero-Copy rendering.

## The Three Pillars of the Renderer

The internal implementation of `WgpuRenderer` relies on three core architectural pillars to achieve its headless, standalone nature.

### Pillar 1: Headless GPU Initialization

To function in environments without a Window (like a server or a CLI), `WgpuRenderer` must forcefully initialize the graphics stack without relying on a surface context.

When you call `WgpuRenderer::new()`, it bypasses the windowing system entirely:

```rust
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::all(),
    display: None, // No window display required!
    // ...
});

let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
    power_preference: wgpu::PowerPreference::HighPerformance,
    compatible_surface: None, // Headless mode
    force_fallback_adapter: false,
}))
.expect("charton: no suitable GPU adapter found");
```

By requesting an adapter with `compatible_surface: None`, Charton ensures it can leverage hardware acceleration even in pure backend environments.

### Pillar 2: The Readback Pipeline & Memory Alignment

GPUs are incredibly fast at drawing pixels, but moving those pixels back to the CPU (System RAM) is notoriously slow and fraught with strict hardware alignment rules.

WGPU dictates that when copying data from a texture to a buffer, the rows of data must be aligned to 256 bytes (`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`). Charton handles this complex padding math seamlessly:

```rust
let bytes_per_pixel = 4usize;
let unpadded_bytes = (width as usize) * bytes_per_pixel;

// WGPU requires buffer rows to be aligned to 256 bytes
let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
let padded_bytes = ((unpadded_bytes + align - 1) / align) * align;
let buffer_size = (padded_bytes * height as usize) as u64;
```

After the GPU finishes rendering and the asynchronous memory map completes, the CPU receives a buffer filled with padded rows. Charton then painstakingly iterates through the buffer, stripping away the empty alignment bytes to yield a clean, contiguous `Vec<u8>` containing pure RGBA data:

```rust
// Strip the WGPU alignment padding to return clean contiguous RGBA data
let data = slice.get_mapped_range();
let mut pixels = Vec::with_capacity(unpadded_bytes * height as usize);

for row in 0..height as usize {
    let start = row * padded_bytes;
    pixels.extend_from_slice(&data[start..start + unpadded_bytes]);
}
```

This guarantees that the array you receive from `.render()` is exactly `width * height * 4` bytes, ready to be encoded directly into a PNG.

### Pillar 3: Deferred Text Compositing (The Hybrid Approach)

Rendering high-quality, anti-aliased text directly on the GPU is exceptionally difficult and often results in bloated shader code. Charton solves this through a brilliant Hybrid Rendering Architecture.

During the GPU pass, Charton only draws the geometry (lines, bars, points, axes lines). Simultaneously, it builds a "Text Ledger"—a list of all strings, their coordinates, and fonts that need to be drawn.

Once the geometric base layer is read back to the CPU, Charton leverages `tiny_skia` (a fast CPU rasterizer) to stamp the text directly onto the pixel buffer:

```rust
// 3. Read back the pixel data from GPU to CPU memory
let mut pixels = self.readback_pixels(&texture, width, height)?;

// 4. Composite text using CPU rasterizer if any text exists in the ledger
if !text_ledger.is_empty() {
    self.composite_text(&mut pixels, width, height, scale_factor, text_ledger)?;
}
```

Inside `composite_text`, the raw `Vec<u8>` is temporarily wrapped in a tiny_skia::Pixmap without copying the data again. The text is drawn perfectly on top, resulting in a flawless final image that combines the blazing speed of GPU geometry with the pristine typography of a CPU text shaping engine.

## The Architectural Boundary: Stopping at the CPU

You might wonder why `WgpuRenderer` doesn't handle uploading the image back to a host GPU. This is a deliberate architectural boundary. Charton's job ends the moment it produces a `Vec<u8>` pixel array. What happens to those bytes next—whether saving them as a PNG, sending them over a server, or uploading them to your app's UI—is entirely up to your application. By leaving the raw data in CPU memory, `WgpuRenderer` achieves universal compatibility, because absolutely any framework or language can read a standard byte array.