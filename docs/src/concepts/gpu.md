# Hybrid GPU-Accelerated Geometry & Deferred Text Rendering Architecture

This document proposes a hybrid, high-performance rendering architecture for our visualization engine. To achieve ultra-high throughput and sub-millisecond interactivity for large-scale datasets, this architecture explicitly splits the rendering pipeline into two decoupled layers: **Pure GPU Instancing for Geometric Primitives** (bypassing CPU-bound vector tessellation like `lyon`) and a **Zero-Allocation Deferred Ledger for Typography** (leveraging target-native text engines like `tiny_skia` and Browser Canvas 2D).

By migrating complex geometry directly into specialized GPU instancing pipelines and reserving text layout for optimal target-specific layers, the engine eliminates heavy CPU preprocessing, avoids font-atlas memory bloat, minimizes WASM binary footprints, and delivers pixel-perfect text anti-aliasing.

---

## Technical Blueprint: Mark Mapping to Low-Level Primitives

The core architecture maps high-level declarative graphics marks onto mathematically optimized low-level `RenderBackend` primitives, executed via decoupled native pathways:

| Chart Element | Low-Level Primitive | Rendering Implementation Blueprint | Core Architectural Advantage |
| :--- | :--- | :--- | :--- |
| **Scatter Plot** | `draw_circle`<br>`draw_polygon` | **Pure GPU**: Instanced SDF (`PointData`) & Template Vertex Pipelines | **Zero CPU Overhead**: Points are batched via Storage Buffers. Vertex expansions and circular SDF boundaries are calculated entirely on-chip, scaling to millions of markers at stable 60 FPS. |
| **Line Chart** | `draw_line`<br>(Polyline) | **Pure GPU**: WGSL Thick Line Shader (Dynamic Extrusion via `Vertex ID`) | **On-Chip Extrusion**: Completely eliminates CPU-side polyline stroke/join calculations. The vertex shader dynamically expands segments into quads on the GPU, ensuring buttery-smooth web interactions. |
| **Area Plot** | `draw_path`<br>(Monotonic) | **Pure GPU**: Linear Triangulation CPU Stream / WGSL Instanced Ribbon Strips | **Trivial Topology**: Statistical area bounds follow strict monotonic X/Y progressions with zero self-intersections. A fast memory pass or GPU strip eliminates complex spatial partitioning. |
| **Map / Geo** | `draw_path`<br>(Complex) | **Pure GPU**: Ahead-Of-Time (AOT) Triangulation via `earcutr` Buffer Cache | **GIS Optimization**: Complex geographical boundaries containing multi-layer holes/islands are tessellated exactly once during data ingestion, streaming static index buffers straight to the GPU. |
| **Text & Labels** | `draw_text` | **Deferred Ledger**: Captured via `Vec<TextConfig>` & Composited at Top-Level | **Target-Native White-Labeling**: Completely strips typography logic out of WGPU. Text is collected via a zero-overhead CPU ledger, then rendered at the very top layer using target-optimized engines (`tiny_skia` / Canvas 2D). |

---

## The Typography Dilemma in Pure WGPU

During our core engineering iteration, rendering text natively inside `wgpu` (via crates like `glyphon` or manual Font Atlas caching) was rejected due to three fatal architectural drawbacks:

1. **WASM Binary & Memory Bloat**: Embedding full font files (`.ttf`/`.otf`) or complex text-shaping engines into WebAssembly dramatically inflates the `.wasm` bundle size, destroying web-native startup times. Furthermore, managing font textures, glyph cache evictions, and UV mappings inside VRAM adds significant runtime complexity.
2. **Subpixel Anti-Aliasing Deficiencies**: Custom GPU text shaders often struggle with subpixel rendering, leading to blurry, jagged, or poorly scaled labels on low-DPI monitors compared to mature operating system font renderers.
3. **Ecosystem Isolation**: For game engines (like *Bevy*) or web apps embedding this library, a hardcoded WGPU text pipeline forces users into a closed ecosystem. It prevents charts from automatically inheriting the host application’s global UI font, scaling rules, and localization/accessibility features.
4. **Extreme Implementation Complexity & Diminishing Returns**: Engineering a robust, production-grade text layout and rendering system within `wgpu` presents an exceptionally steep learning curve and massive development overhead. Our foundational iterations yielded only partial success and fell significantly short of our core visual expectations. The staggering effort required to manually handle layout bounds, vertical baseline alignment, and multi-language text shaping creates an unsustainable engineering bottleneck with severe diminishing returns.

---

## The Solution: Hybrid Layered Compositing (Deferred Ledger Mode)

To circumvent these issues, `WgpuBackend` acts as a pure geometric powerhouse. When a `draw_text` call is triggered, the backend performs **zero GPU allocations and creates no vertices**. Instead, it acts as a lightweight "accountant," pushing the raw configurations into a serial ledger: `pub collected_texts: Vec<TextConfig>`.

Once the WGPU pipeline finishes flushing the geometric primitives to the target buffer, a top-level orchestrator takes over, processing the text ledger through a **Target-Aware Dual Engine Pipeline**:

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

### 1. WebAssembly Environment (WASM / Browser Target)
* **Execution**: `wgpu` renders the grid lines, ticks, and geometric marks directly onto a WebGL2 or WebGPU `<canvas>`.
* **Text Layer**: The engine acquires the native browser's `CanvasRenderingContext2d` on a transparent top-layer canvas or overlays HTML/Canvas text directly. It loops through `collected_texts` and invokes native `ctx.fill_text(...)`.
* **Advantage**: Bypasses the entire text rendering stack. It leverages the browser’s highly optimized, hardware-accelerated typography engine for free, gaining perfect anti-aliasing, layout shaping, internationalization (i18n), and seamless integration with standard web fonts.

### 2. Desktop / Headless Environment (Local PNG Export)
* **Execution**: `wgpu` renders the high-density geometric marks into an off-screen texture buffer, which is then copied back to a CPU pixel array.
* **Text Layer**: The engine initializes a `tiny_skia` raster backend over the pixel buffer. It iterates over the `collected_texts` ledger and stamps the text labels cleanly on top of the WGPU-generated geometric base image.
* **Advantage**: Retains full standalone rendering capability for server-side automated reports, command-line interfaces, and unit testing without requiring an active browser or UI windowing subsystem.

---

## Alternative Evaluation: The Case for `lyon`

*(This section remains retained as our core justification for choosing custom shaders over CPU vector solvers for geometry processing.)*

### Where `lyon` Excels
`lyon` is an exceptionally robust framework designed primarily for arbitrary, unstructured, and unpredictable 2D vector graphics, such as:
* **SVG Engines / Web Browsers**: Processing arbitrary vector source files filled with nested Bézier curves and dynamic stroke-joins where geometry is completely unknown prior to runtime.
* **Vector Design Tools** (e.g., Figma clones): Where users freely draw, twist, and overlap vector paths.

### Why We Reject Tessellation for Geometry
For a specialized data visualization engine, utilizing a general-purpose vector library introduces severe structural inefficiencies:
* **Main-Thread Bottlenecks**: Data visualization deals with highly structured, mathematically uniform data (e.g., uniform scatter markers, layout-confined bar quads). Utilizing a comprehensive vector solver to parse these highly predictable primitives introduces massive, redundant CPU branching, choking single-threaded WebAssembly environments.
* **Embracing Hardware Parallelism**: By rejecting CPU-side preprocessing, we transfer tasks (such as thick line expansions and marker geometry creation) directly into parallelized GPU processing cores via custom WGSL shaders. The CPU is liberated, acting purely as a rapid data pipe streaming raw structured buffers straight into VRAM.

---

## Conclusion

By implementing this Hybrid Layered Architecture, our system achieves the best of both worlds: **uncompromised, chart-dropping GPU acceleration for geometric data points**, and **zero-cost, ultra-crisp, flexible typography via native host engines**. The resulting pipeline is remarkably lean, deterministic, and tailored for fluid interactivity under extreme industrial data densities.
