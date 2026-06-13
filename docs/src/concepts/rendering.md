# Rendering Primitives & Backend Contract

This chapter defines the unified, internal rendering contract that powers all Charton backends (SVG, PNG/Raster, PDF, WGPU). These primitives are not exposed to end users—they form the low-level geometry layer that translates declarative marks (Point, Line, Boxplot, Text, Area) into pixels or vectors.

The design follows strict principles of semantic clarity, cross-backend consistency, performance optimization, and implementation simplicity. Every method has a single, well-defined responsibility.

## Core Design Principles

Charton’s `RenderBackend` is engineered around five non-negotiable constraints:

1. Backend Agnosticism: A single implementation of marks (Point, Line, Boxplot, Area) must work identically across vector (SVG, PDF) and raster (PNG, WGPU) targets without modification. The same input produces identical visual output regardless of backend.

2. Semantic Separation & Data-Driven Routing: Each drawing method corresponds to a distinct geometric category. We do not force primitives to guess intents. Complex geometries are routed automatically via lightweight topology hints (e.g., `PathTopology`), ensuring the backend always selects the optimal hardware pathway.

3. Cross-Backend Semantic Consistency: All backends must implement exactly the same set of primitives with identical semantics. What `draw_polygon` means in SVG is exactly what it means in PNG and WGPU. This is the foundation of Charton's reliability.

4. Performance by Design (The 4-Tier Architecture): Primitives are optimized for their intended use cases: instanced SDF for circles on GPU, line extrusion for fast strokes, hardware Triangle Fans for convex polygons, and heavy Stencil-Then-Cover (STC) pipelines solely reserved for highly complex maps.

5. Deferred Typography: Font rendering and glyph layout are highly platform-dependent. The backend contract treats text as a "Deferred Ledger," collecting text commands during the geometric pass and executing them natively via the host environment (HTML5 Canvas 2D or CPU-composited Skia) to guarantee pixel-perfect legibility and zero GPU-atlas overhead.

## The RenderBackend Primitives: Semantics & Usage

Below is the formal definition of each primitive, its semantic role, performance characteristics, and intended use cases across all backends.

1. `draw_circle(&mut self, config: CircleConfig)`
- Semantics: Renders a perfect circle defined by a center point and radius.
- Implementation:
    - Vector: Native <circle> primitive.
    - CPU Raster: Path-built vector circles rendered via anti-aliased scan-conversion.
    - GPU Raster (WGPU): Instanced SDF (Signed Distance Field) shader evaluated on a single quad.

- Performance: Extremely fast. Constant-time shader math on GPU. Ideal for millions of scatter plot markers.
- Use Cases: Scatter points, outlier dots in boxplots, radar chart vertices.

2. `draw_rect(&mut self, config: RectConfig)`
- Semantics: Draws an axis-aligned rectangle from position, width, and height, with optional rounded corners.
- Implementation:
    - Vector: Native `<rect>` primitive with `rx/ry` attributes.
    - CPU Raster: Optimized direct pixel bounding-box fill.
    - GPU Raster (WGPU): Instanced quad with fragment shader clipping for rounded corners.
- Performance: Near-optimal. Simple bounds check, zero complex geometry generation. Supports millions of instances.
- Use Cases: Bar marks, boxplot bodies, heatmap cells, UI backgrounds.

3. `draw_line(&mut self, config: LineConfig)`
- Semantics: Draws a single, straight line segment between two explicit points.
- Implementation:
    - Vector: Native `<line>` primitive.
    - GPU Raster (WGPU): Expanded directly to an instanced quad in the WGSL vertex shader.
- Performance: Minimal overhead. Bypasses traditional line width limitations.
- Use Cases: Boxplot whiskers, axis ticks, grid strokes, error bars.

4. `draw_gradient_rect(&mut self, config: GradientRectConfig)`
- Semantics: Fills an axis-aligned rectangle with a seamless linear gradient.
- Implementation:
    - Vector: Context-linked `<linearGradient>`.
    - GPU Raster (WGPU): Custom fragment shader computing interpolations directly on instanced quads.

5. `draw_polygon(&mut self, config: PolygonConfig)`
- Semantics: Renders a closed, strictly convex polygon used for high-performance internal area filling. 
- Implementation:
    - Vector: Closed `<polygon>` element.
    - GPU Raster (WGPU): Fast-path Triangle Fan hardware rasterization. Single pass, zero tessellation or stencil overhead.
- Performance: Extremely fast, but relies on the CPU/Mark layer to guarantee the input is convex. Not intended for arbitrary or concave geo-polygons.
- Use Cases:
    - Symmetric markers (Triangle, Diamond, Hexagon, Star).
    - Area Plots: The continuous concave area is sliced by the CPU into hundreds of perfect, convex trapezoids ($X_n \le X_{n+1}$) and fed sequentially to this fast-path filler.

6. `draw_path(&mut self, config: PathConfig)`
- Semantics: The universal topological router. Renders a continuous polyline or a complex area boundary. Its behavior is strictly dictated by the `topology` hint.
- Implementation:
    - Vector: Continuous `<path>` or `<polyline>` element.
    - GPU Raster (WGPU): Acts as a dispatcher:
        - `PathTopology::Simple` -> `draw_path_simple`: Pure GPU normal extrusion. **Strokes only, no fills**. Used for smoothing chart boundaries and polylines.
        - `PathTopology::Complex` -> `draw_path_complex`: The heavy-duty Stencil-Then-Cover (STC) pipeline. Handles concave, self-intersecting, or holed polygons seamlessly using Odd-Even winding hardware passes.
- Performance: Provides a zero-cost abstraction. Simple paths bypass expensive triangulation, while complex paths safely fallback to robust stencil hardware logic.
- Use Cases: Continuous line marks, the top-highlighted edge of an area plot (`Simple`), geographic maps, and multi-layered complex boundaries (`Complex`).

7. `draw_text(&mut self, config: TextConfig)`
- Semantics: Registers formatted text with strict layout attributes (text-anchor, dominant-baseline) into a Deferred Text Ledger.
- Implementation:
    - Vector (SVG/PDF): Native `<text>` nodes.
    - Desktop/Headless (WGPU + PNG): Text commands are collected and bypassed during the WGPU pass. After reading the WGPU rendered buffer back to the CPU, `tiny-skia` handles the typography compositing.
    - WASM/Web (WGPU + Canvas2D): The WGPU virtual surface renders the geometry. Text is deferred to an absolute-positioned HTML5 `<canvas>` overlay, utilizing the browser's highly optimized `CanvasRenderingContext2D` for pixel-perfect native font rendering.
- Performance: Eliminates WebAssembly-to-JS font glyph memory roundtrips, avoids GPU atlas bloat, and guarantees pristine native text antialiasing across all operating systems.
- Use Cases: Data labels, axis labels, titles, and legends.

## Mark-to-Primitive Routing Rules
The real power of this design is how high-level declarative marks map to low-level primitives. This routing is deterministic, optimized, and identical across all backends:

| Mark Type       | Primary Primitive Mapping                              | Backend Execution Strategy                                                                 |
| --------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| PointMark       | `draw_circle`, `draw_rect`, `draw_polygon`                    | GPU Instanced SDF & Hardware Convex Fan.                                                   |
| LineMark        | `draw_path` (`Topology::Simple`)                           | GPU On-Chip Vertex Extrusion (Stroke only).                                                |
| AreaMark        | `draw_polygon` (Fill) + `draw_path` (Simple Stroke)        | CPU slices concave area into convex trapezoids for draw_polygon; top boundary routed to draw_path for clean extrusion. |
| BoxplotMark     | `draw_rect`, `draw_line`, `draw_circle`                      | Decomposed cleanly into atomic backend shape calls.                                        |
| BarMark         | `draw_rect`                                              | Highly optimized direct bounding-box rectangle fills.                                      |
| Geographic      | `draw_path` (`Topology::Complex`)                          | STC (Stencil-Then-Cover) dual-pass GPU filling for irregular concave shapes with holes.     |
| TextMark        | `draw_text` (Deferred Ledger)                            | Geometry rendered via GPU, typography composited via host native environment (HTML5 Canvas 2D / tiny-skia). |

## WGPU Implementation Notes (For Backend Developers)
The WGPU backend (`WgpuBackend`) strictly adheres to a 4-Tier Geometry Architecture mixed with a Deferred Compositing pipeline:

1. Tier 1 (SDF Instancing): `circle_pipeline`, `rect_pipeline` for mathematically perfect, zero-tessellation primitives.
2. Tier 2 (Vertex Extrusion): `draw_path_simple` uses normal-expansion in WGSL to create resolution-independent thick lines.
3. Tier 3 (Convex Fast-Path): `draw_polygon` relies on basic `TriangleList`/`TriangleStrip` topology for blindingly fast fills, assuming the CPU has provided convex indices.
4. Tier 4 (STC Heavy Weapon): `draw_path_complex` utilizes `wgpu::StencilState` with `StencilOperation::Invert` (Odd-Even winding) to solve arbitrary geographic concave polygons directly on the GPU.

**The Deferred Text Ledger:**

To decouple heavy font-shaping from the GPU, `render_primitive_only` collects all `TextConfig` calls into a `Vec<TextConfig>`. This ledger is returned to the host runner (`save_wgpu_png` or `render_to_canvas`), which then safely overlays the text using the most optimal native 2D API available to the platform.