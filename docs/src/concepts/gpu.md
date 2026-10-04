# GPU Architecture & the WgpuRenderer

`WgpuRenderer` is Charton's standalone, headless GPU backend. It takes a
declarative chart and returns a raw `Vec<u8>` RGBA pixel buffer, encapsulating
the whole WGPU lifecycle. This chapter covers **when** to use it, **how** it maps
marks to low-level primitives, and the three architectural pillars it is built
on.

## The golden rule: use cases and anti-patterns

`WgpuRenderer` is a **black box**: data in, pixels out. It is ideal for:

- CLI tools and scripts that generate charts in a terminal.
- Server-side rendering — static images produced on a backend.
- Static exports to `.png` / `.svg` on disk.
- Decoupled GUIs (for example `winit`) where isolating the WGPU dependency
  matters more than raw framerate.

> **Anti-pattern:** do **not** call `renderer.render()` inside a 60 FPS loop
> (a Bevy system, an egui immediate-mode UI). Every call does a full GPU→CPU
> readback, allocates new textures and blocks the thread. For interactive GUIs,
> inject your app's existing WGPU context with
> `chart.render_to_surface(&mut app_backend, &app_view)` for zero-copy rendering
> (see [Zero-Copy Rendering](../gui/zero_copy.md)).

## The technical blueprint: marks → primitives

The architecture maps high-level marks onto mathematically optimised primitives,
each executed through the best pathway for its shape:

| Chart element | Primitive | GPU / CPU pathway | Why |
|---|---|---|---|
| Scatter | `draw_circle`, `draw_polygon` | Instanced SDF (`PointData`), storage buffers | Zero CPU overhead; millions of markers at 60 FPS |
| Line | `draw_line` (polyline) | WGSL thick-line shader (extrusion by `Vertex ID`) | On-chip extrusion; no CPU stroke/join math |
| Area | `draw_path` (monotonic) | Linear triangulation / instanced ribbon strips | Trivial topology, no self-intersection |
| Map / geo | `draw_path` (complex) | Ahead-of-time triangulation (`earcutr`) cache | Tessellate once at ingestion, stream index buffers |
| Text | `draw_text` | Deferred ledger, composited on top | Target-native typography (see Pillar 3) |

## Pillar 1: Headless GPU initialization

To work without a window (a server, a CLI), the instance is created without a
surface:

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

Requesting an adapter with `compatible_surface: None` keeps hardware acceleration
available in pure backend environments.

## Pillar 2: The readback pipeline & memory alignment

Moving pixels from the GPU back to system RAM is slow and subject to strict
alignment: WGPU requires buffer rows to be padded to 256 bytes
(`COPY_BYTES_PER_ROW_ALIGNMENT`). Charton does the padding math so you never see
it:

```rust
let bytes_per_pixel = 4usize;
let unpadded_bytes = (width as usize) * bytes_per_pixel;

// WGPU requires buffer rows to be aligned to 256 bytes
let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
let padded_bytes = ((unpadded_bytes + align - 1) / align) * align;
let buffer_size = (padded_bytes * height as usize) as u64;
```

After the readback it strips the padding to yield clean, contiguous RGBA:

```rust
let data = slice.get_mapped_range();
let mut pixels = Vec::with_capacity(unpadded_bytes * height as usize);
for row in 0..height as usize {
    let start = row * padded_bytes;
    pixels.extend_from_slice(&data[start..start + unpadded_bytes]);
}
```

The array you get back is exactly `width * height * 4` bytes, ready to encode as
a PNG.

## Pillar 3: Deferred text compositing (hybrid typography)

Rendering high-quality, sub-pixel, multi-language text directly on the GPU is
hard and brittle. Font files bloat the WASM bundle; custom shaders rarely match
an OS font renderer; and hard-coding a GPU text pipeline forces host apps
(Bevy, egui, a browser) into a closed ecosystem. Rather than fight this, the
renderer **defers** all text.

During the GPU pass it draws only geometry and, for every `draw_text`, pushes a
`TextConfig` into a ledger (`Vec<TextConfig>`) — zero GPU allocation, no
vertices. After the geometry is read back to the CPU, a top-level orchestrator
composites the ledger through a **target-aware dual-engine pipeline**:

```text
[Declarative Chart Request]
         │
         ├──► Geometry ──► WgpuBackend (Instanced SDF / WGSL Shaders) ──► GPU Target Surface
         │                                                                      │
         └──► Text      ──► Collected Memory Ledger (Vec<TextConfig>)           │  (Compositing)
                                       │                                        ▼
                        ┌──────────────┴──────────────┐                 ┌──────────────┐
                        ▼                             ▼                 │              │
               [Desktop / Headless]             [WASM / Web]            │              │
                        │                             │                 │              │
                  (tiny_skia)                 (Canvas 2D Context)       │              │
                        │                             │                 │              │
                        ▼                             ▼                 ▼              ▼
               Stamp Text on Bitmaps          Ctx.fill_text Overlays ──►[Final Visual Output]
```

- **Desktop / headless**: `tiny_skia` is initialised over the read-back pixel
  buffer and stamps the labels onto the GPU-rendered base image. Server-side
  reports, CLIs and tests work with no window and no browser.
- **WASM / browser**: the geometry goes to a WebGL2/WebGPU `<canvas>`, and the
  ledger is drawn with the browser's native `CanvasRenderingContext2d`. This
  gives perfect anti-aliasing, text shaping and i18n for free, and inherits the
  page's fonts.

```rust
// Read back the geometry, then composite text if the ledger is non-empty.
let mut pixels = self.readback_pixels(&texture, width, height)?;
if !text_ledger.is_empty() {
    self.composite_text(&mut pixels, width, height, scale_factor, text_ledger)?;
}
```

## The architectural boundary: stopping at the CPU

Charton's job ends when it produces the `Vec<u8>` pixel array. It deliberately
does **not** upload the image back to a host GPU: what happens next — saving a
PNG, streaming it over HTTP, or uploading it into your app's UI — is up to the
application. Leaving the data in CPU memory gives universal compatibility,
because any framework or language can read a byte array.

## Why not `lyon` (CPU tessellation)?

`lyon` is excellent for arbitrary, unpredictable vector graphics — SVG engines,
vector design tools, anything with free-form Béziers and dynamic stroke joins.
For a *data visualization* engine we reject CPU tessellation:

- **Main-thread bottleneck.** Chart data is highly structured and uniform
  (scatter markers, layout-confined bar quads). Running a general-purpose vector
  solver over it wastes CPU on branching, which is fatal in single-threaded
  WebAssembly.
- **Embrace hardware parallelism.** Pushing stroke expansion and marker geometry
  into WGSL shaders turns the CPU into a fast pipe streaming structured buffers
  straight into VRAM.

Only genuinely complex geometry — map boundaries with holes and islands — is
triangulated ahead of time (`earcutr`) and cached, because that shape is not
uniform.

## Conclusion

The hybrid layered architecture gives both halves: **GPU acceleration for
geometry** (markers, lines, areas via instanced shaders) and **zero-margin,
crisp typography via native host engines** (tiny_skia / Canvas 2D). The result is
lean, deterministic and built for interactivity under extreme data densities.

## See also

- [Rendering Backends](rendering.md) — the `RenderBackend` primitive contract.
- [Zero-Copy Rendering](../gui/zero_copy.md) — integrating an existing WGPU
  context for interactive apps.
- [WASM WGPU Blazing-Fast Rendering](../web/wasm_gpu_wgpu.md) — the browser
  walkthrough.
