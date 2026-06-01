# Pure GPU-Accelerated Rendering Architecture

This document proposes a pure GPU rendering architecture powered by `wgpu` for our  visualization engine. To achieve ultra-high throughput and sub-millisecond interactivity for large-scale datasets in WebAssembly (WASM) environments, this architecture explicitly bypasses general-purpose CPU-bound vector tessellation libraries (such as `lyon`).

By migrating geometric primitives directly into specialized GPU instancing pipelines and custom WGSL shaders, the engine eliminates heavy CPU preprocessing bottlenecks, minimizes WASM binary footprints, and establishes an optimal, zero-readback execution stream while maintaining strict semantic alignment with our lightweight CPU raster targets.

---

## Technical Blueprint: Mark Mapping to Low-Level Primitives

The core architecture maps high-level declarative graphics marks directly onto mathematically optimized low-level `RenderBackend` primitives, executed natively via specialized hardware pathways:

| Chart Element | Low-Level Primitive | Pure GPU Implementation Blueprint | Core Architectural Advantage |
| :--- | :--- | :--- | :--- |
| Scatter Plot | `draw_circle`<br>`draw_polygon` | Instanced SDF (`PointData`) & Template Vertex (`PolygonData`) Pipelines | Zero CPU Overhead: Points are batched via Storage Buffers. Vertex expansions and circular SDF boundaries are calculated entirely on-chip, scaling to millions of markers at stable 60 FPS. |
| Line Chart | `draw_line`<br>(Polyline) | Pure WGSL Thick Line Shader (Dynamic Extrusion via `Vertex ID`) | On-Chip Extrusion: Completely eliminates CPU-side polyline stroke/join calculations. The vertex shader dynamically expands segments into quads on the GPU, ensuring buttery-smooth web interactions. |
| Area Plot | `draw_path`<br>(Monotonic) | Linear Triangulation CPU Stream / WGSL Instanced Ribbon Strips | Trivial Topology: Statistical area bounds follow strict monotonic X/Y progressions with zero self-intersections. A fast memory pass or GPU strip eliminates complex spatial partitioning. |
| Map / Geo | `draw_path`<br>(Complex) | Ahead-Of-Time (AOT) Triangulation via `earcutr` Buffer Cache | GIS Optimization: Complex geographical boundaries containing multi-layer holes/islands are tessellated exactly once during data ingestion, streaming static index buffers straight to the GPU. |
| Text & Labels| `draw_text` | Native Integration with the `glyphon` Canvas Pipeline | Pure GPU Text Layer: Glyph textures are composited directly into the active `wgpu` render pass, eliminating intermediate CPU canvases and expensive WASM-to-JS pixel copying. |

---

## Alternative Evaluation: The Case for `lyon`

During our foundational architectural evaluation phase, `lyon` (the industry-standard vector tessellation crate in Rust) was thoroughly analyzed as a potential geometry solver.

### Where `lyon` Excels
`lyon` is an exceptionally robust, production-proven framework designed primarily for arbitrary, unstructured, and unpredictable 2D vector graphics, such as:
* SVG Rendering Engines / Web Browsers: Processing arbitrary vector source files filled with nested Bézier curves, dynamic stroke-joins, and infinite self-intersecting clip paths where geometry is completely unknown prior to runtime.
* Vector Design Application Ecosystems (e.g., Figma/Illustrator clones): Where users freely draw, twist, overlap vector paths, and punch unpredictable layout holes using pen tools.
* General-Purpose UI Frameworks (e.g., Iced, Vello): Managing arbitrary layout clipping boundaries, complex control states, and variable rounded interface components.

In those native scenarios, `lyon`'s heavy-duty topological layout engines (Even-Odd/Non-Zero filling math, curve-to-line approximations, and ray-casting intersections) are mathematically non-negotiable.

### Why We Reject the Tessellation Approach

For a specialized data visualization engine, utilizing a general-purpose vector library like `lyon` introduces severe structural inefficiencies. We have chosen a Pure GPU/Shader approach due to three fatal architectural drawbacks of the traditional CPU-tessellation pipeline:

#### Over-Engineering & Main-Thread CPU Bottlenecks
Data visualization deals with highly structured, mathematically uniform data (e.g., monotonic polylines, uniform scatter markers, layout-confined bar quads). Utilizing a comprehensive vector solver to parse these highly predictable primitives introduces massive, redundant CPU branching. When rendering datasets containing hundreds of thousands of entries, a CPU-bound geometry generator completely chokes the main thread—rendering smooth rendering impossible within single-threaded WebAssembly environments.

#### Binary Footprint Bloat in WebAssembly
`lyon`'s extensive geometric and algebraic logic introduces notable footprint overhead into compiled artifacts. For a web-native visualization engine, keeping bundle sizes strictly minimal is a high-priority constraint. Shifting to an instanced shader model drastically cuts the compiled `.wasm` file size, ensuring significantly faster initial web asset load times and immediate drawing responses.

#### Embracing Hardware Parallelism (The Paradigm Shift)
Statistical graphics rendering is inherently a problem of high data throughput over geometric versatility. By rejecting heavy CPU-side preprocessing, we transfer structural tasks (such as thick line cap/join expansion and marker geometry creation) directly into parallelized GPU processing cores via custom WGSL shaders. The CPU is fully liberated from geometry processing, acting purely as a rapid data pipe that streams raw structured buffers directly into VRAM.

---

## Conclusion

By executing a pure GPU Architecture, our system aims to bypasses the expensive, frame-dropping "CPU-tessellate-then-GPU-render" workflow loop. The resulting pipeline is remarkably lean, deterministic, and hardware-native—purpose-built to maintain fluid interactivity under extreme industrial data densities.