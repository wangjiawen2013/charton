# Rendering Primitives & Backend Contract

This chapter defines the unified backend contract that powers all Charton backends (SVG, PNG/Raster, PDF, WGPU). These primitives are the backend-implementer API, not the chart-author API: chart authors work with marks, while backends translate those marks into pixels or vectors.

The design follows strict principles of semantic clarity, cross-backend consistency, performance optimization, and implementation simplicity. Every method has a single, well-defined responsibility.

## Core Design Principles

Charton’s `RenderBackend` is engineered around five non-negotiable constraints:

1. Backend Agnosticism: A single implementation of marks (Point, Line, Boxplot, Area) must work identically across vector (SVG, PDF) and raster (PNG, WGPU) targets without modification. The same input produces identical visual output regardless of backend.

2. Semantic Separation & Data-Driven Routing: Each drawing method corresponds to a distinct geometric category. We do not force primitives to guess intents. Complex geometries are routed automatically via lightweight topology hints (e.g., `PathTopology`), ensuring the backend always selects the optimal hardware pathway.

3. Cross-Backend Semantic Consistency: All backends must implement exactly the same set of primitives with identical semantics. What `draw_path` means in SVG is exactly what it means in PNG and WGPU. This is the foundation of Charton's reliability.

4. Performance by Design (The 4-Tier Architecture): Primitives are optimized for their intended use cases: instanced SDF for circles on GPU, line extrusion for fast strokes, hardware triangle fans for convex paths, and stencil-then-cover for concave or multi-ring paths.

5. Deferred Typography: Font rendering and glyph layout are highly platform-dependent. The backend contract treats text as a "Deferred Ledger," collecting text commands during the geometric pass and executing them natively via the host environment (HTML5 Canvas 2D or CPU-composited Skia) to guarantee pixel-perfect legibility and zero GPU-atlas overhead.

## The RenderBackend Primitives: Semantics & Usage

Below is the formal definition of each primitive, its semantic role, performance characteristics, and intended use cases across all backends.

1. `draw_circle(&mut self, config: CircleConfig)`
- Semantics: Renders a perfect circle defined by a center point and radius.
- Implementation:
    - Vector: Native `<circle>` primitive.
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

5. `draw_path(&mut self, config: PathConfig)`
- Semantics: The universal vector-path primitive. A path contains one or more subpaths (rings). Its behavior is dictated by the `topology` hint:
    - `PathTopology::Simple`: open polylines. Every subpath is stroked independently; nothing is closed or filled.
    - `PathTopology::Convex`: one closed, convex ring. Backends may use a triangle fan or another convex fast path.
    - `PathTopology::Complex`: one or more closed rings, filled with the even-odd rule. Concave shapes, self-intersections and holes are all expressed by the subpath list; an inner ring simply flips the parity to zero.
- Implementation:
    - Vector: `<path>` for multi-subpath paths, or a smaller `<polygon>` for a single convex ring.
    - GPU Raster (WGPU): Acts as a dispatcher:
        - `Simple` -> normal-extrusion stroke shader.
        - `Convex` -> direct triangle-fan fill (the convex fast path).
        - `Complex` -> stencil-then-cover, one stencil fan per ring and one cover pass for the combined bounding box.
- Use Cases: Line marks, area fill and boundary, point glyphs (convex or star), violins, maps, and any path with holes.

6. `draw_text(&mut self, config: TextConfig)`
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
| PointMark       | `draw_circle`, `draw_rect`, `draw_path`                | Instanced SDF for circles, rect fast path for squares, triangle fan for convex glyphs, STC for the concave star. |
| LineMark        | `draw_path` (`Simple`)                                 | GPU on-chip vertex extrusion (stroke only).                                                |
| AreaMark        | `draw_path` (`Complex` fill, `Simple` top stroke)       | Even-odd fill for the closed area; the top boundary is a separate open path.                |
| BoxplotMark     | `draw_rect`, `draw_line`, `draw_circle`                | Decomposed cleanly into atomic backend shape calls.                                        |
| BarMark         | `draw_rect`                                            | Highly optimized direct bounding-box rectangle fills.                                      |
| Geographic      | `draw_path` (`Complex`)                                | Multi-ring STC fill: outer ring plus holes as separate subpaths.                           |
| TextMark        | `draw_text` (Deferred Ledger)                          | Geometry rendered via GPU, typography composited via host native environment (HTML5 Canvas 2D / tiny-skia). |

## WGPU Implementation Notes (For Backend Developers)
The WGPU backend (`WgpuBackend`) strictly adheres to a 4-Tier Geometry Architecture mixed with a Deferred Compositing pipeline:

1. Tier 1 (SDF Instancing): `circle_pipeline`, `rect_pipeline` for mathematically perfect, zero-tessellation primitives.
2. Tier 2 (Vertex Extrusion): the `Simple` path pipeline uses normal-expansion in WGSL to create resolution-independent thick lines.
3. Tier 3 (Convex Fast-Path): `draw_path` with `PathTopology::Convex` uses a triangle fan for blindingly fast fills. The mark layer guarantees the single ring is convex.
4. Tier 4 (STC Heavy Weapon): `draw_path` with `PathTopology::Complex` uses `wgpu::StencilState` with `StencilOperation::Invert` (odd-even winding). One stencil fan per ring makes holes and arbitrary concave shapes fall out of the parity rule.

**The Deferred Text Ledger:**

To decouple heavy font-shaping from the GPU, `render_primitive_only` collects all `TextConfig` calls into a `Vec<TextConfig>`. This ledger is returned to the host runner (`save_wgpu_png` or `render_to_canvas`), which then safely overlays the text using the most optimal native 2D API available to the platform. Font families, fallback and error handling for that overlay are described in [Fonts & Text Rendering](fonts.md).
## Semantic Scopes: the Sixth Contract

The six drawing methods above answer *what shape* and *where*. They cannot
answer *what it means*: `draw_line` draws a grid line, a tick, an axis spine and
a data mark identically. Structuring the document therefore needs one more
concept, and it deliberately is not a new primitive.

```rust
fn begin_group(&mut self, group: &Group<'_>);
fn end_group(&mut self);
```

A `Group` (see `core::semantic`) carries a `Role` — `Axis(X)`, `TickLabel`,
`Grid`, `Mark`, `Legend`, … — plus optional identity (`layer`, `mark_type`,
`series`) and an optional clip rectangle. Every group opened is closed, and
groups nest.

What each backend does with it:

| Backend | Effect |
|---|---|
| SVG | `<g class="charton-…" data-… aria-… clip-path="…">` |
| PDF | applies the clip rectangle and, when `PdfOptions` enables tagging, starts a marked-content section for each whole accessible unit (see `Role::structure`) |
| Raster | swaps the clipping mask, restoring the enclosing one on close |
| WGPU | narrows the scissor rectangle, restoring the enclosing one on close |

Two properties keep this compatible with the streaming design:

* **Roles are groups, not per-primitive tags.** The number of groups is
  O(ticks + layers), not O(points), so nothing is allocated per data point.
* **A clip is a property of a group.** There is exactly one way to isolate a
  region, and therefore exactly one `<defs>` block per document.

Per-observation identity is the one piece that does travel on a primitive
(`begin_element`/`end_element` → `data-value`): a bar or a tick label is exactly
the element a stylesheet wants to address, and a value on a wrapping group could
not override the presentation attributes of the element it wraps.

Because the semantics live in the pipeline rather than in the SVG backend, the
same stream drives PDF tagging and can never drift between backends.

See [Semantic Output: CSS, Accessibility & Tagging](../export/semantic_output.md)
for the class grammar, the `SvgOptions`/`PdfOptions` APIs and the PDF structure
mapping.
