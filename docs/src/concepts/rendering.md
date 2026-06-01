# Rendering Primitives & Backend Contract

This chapter defines the unified, internal rendering contract that powers all Charton backends (SVG, PNG/Raster, PDF, WGPU). These primitives are not exposed to end users—they form the low-level geometry layer that translates declarative marks (Point, Line, Boxplot, Text, Area) into pixels or vectors.

The design follows strict principles of semantic clarity, cross-backend consistency, performance optimization, and implementation simplicity. Every method has a single, well-defined responsibility.

---

# Core Design Principles

Charton’s RenderBackend is engineered around five non-negotiable constraints:

## Backend Agnosticism
A single implementation of marks (Point, Line, Boxplot, Area) must work identically across vector (SVG, PDF) and raster (PNG, WGPU) targets without modification. The same input produces identical visual output regardless of backend.

## Semantic Separation
Each drawing method corresponds to a distinct geometric category. No method tries to represent multiple visual concepts. This eliminates ambiguity for both backend implementors and mark authors.

## Cross-Backend Semantic Consistency
All backends must implement exactly the same set of primitives with identical semantics. What `draw_polygon` means in SVG is exactly what it means in PNG (via `tiny-skia`) and WGPU. This is the foundation of Charton's reliability.

## Performance by Design
Primitives are optimized for their intended use case: instanced SDF for circles on GPU, direct CPU bounding-box fills for rectangles on CPU, CPU precomputed regular vertices for symmetric markers, specialized on-chip vertex extrusion for thick web lines, and native font parsing/atlas compositing for text labels.

## Minimalism & Completeness
The interface provides only the necessary primitives to build every chart in the grammar—no bloat, no missing building blocks. Every method exists for a specific, well-justified reason.

---

# The RenderBackend Primitives: Semantics & Usage

Below is the formal definition of each primitive, its semantic role, performance characteristics, and intended use cases across all backends.

---

## 1. `draw_circle(&mut self, config: CircleConfig)`

### Semantics
Renders a perfect circle defined by a center point and radius.

### Implementation
- **Vector backends (SVG, PDF)**: Native `<circle>` primitive.
- **CPU Raster backend (PNG)**: Path-built vector circles rendered via `tiny_skia::PathBuilder::push_circle` with anti-aliasing.
- **GPU Raster backend (WGPU)**: Instanced SDF (signed distance field) shader evaluated on a single quad.

### Performance
- **Extremely fast**: Constant-time shader math on GPU, and optimized circle scan-conversion on CPU. No complex runtime tessellation required.
- Ideal for millions of scatter plot markers.

### Use Cases
- Point marks (circular style)
- Outlier points in boxplots
- Circular markers and dots

---

## 2. `draw_rect(&mut self, config: RectConfig)`

### Semantics
Draws an axis-aligned rectangle from position, width, and height, with optional rounded corners.

### Implementation
- **Vector backends (SVG, PDF)**: Native `<rect>` primitive with `rx`/`ry` attributes.
- **CPU Raster backend (PNG)**: Highly optimized direct pixel filling via `tiny_skia::Pixmap::fill_rect`.
- **GPU Raster backend (WGPU)**: Instanced quad with fragment shader clipping for rounded corners.

### Performance
- **Near-optimal performance**: Simple bounds check, zero complex geometry generation. Supports millions of instances for heatmaps and bar charts.

### Use Cases
- Boxplot boxes
- Bar marks
- Axis rectangles
- Heatmap cells
- UI panels and backgrounds

---

## 3. `draw_polygon(&mut self, config: PolygonConfig)`

### Semantics
Renders a closed, convex, regular polygon used exclusively for symmetric visual markers (e.g., standard scatter shapes).

### Implementation
- **Vector backends (SVG, PDF)**: Closed `<polygon>` element generated from regular vertices.
- **CPU Raster backend (PNG)**: Linear path sequence that explicitly calls `pb.close()` before undergoing `fill_path` or `stroke_path` under `FillRule::Winding`.
- **GPU Raster backend (WGPU)**: CPU-precomputed shape templates uploaded once and rendered as highly efficient instanced geometry.

### Performance
- Fast for symmetric marker shapes: vertex generation happens once at configuration/startup, never per frame.
- Perfect for thousands to millions of non-circular point markers. Not intended for arbitrary or concave geo-polygons—those belong to `draw_path`.

### Use Cases
- Point markers: Triangle, Diamond, Pentagon, Hexagon, Star, Octagon
- Symmetric small shapes in legends and annotations

---

## 4. `draw_line(&mut self, config: LineConfig)`

### Semantics
Draws a single, straight line segment between two explicit points. Supports dashed arrays.

### Implementation
- **Vector backends (SVG, PDF)**: Native `<line>` primitive with `stroke-dasharray`.
- **CPU Raster backend (PNG)**: Linear vector path segment rendered using `tiny_skia::Stroke` accompanied by `StrokeDash`.
- **GPU Raster backend (WGPU)**: Expanded directly to an instanced quad in the WGSL vertex shader for consistent line width across all scales, bypassing traditional line width limitations.

### Performance
- Minimal overhead: No complex path expansion or topological sorting.
- Ideal for thousands of discrete line segments like grid lines, ticks, and error bars.

### Use Cases
- Boxplot whiskers and median lines
- Axis ticks and simple grid strokes
- Error bars

---

## 5. `draw_path(&mut self, config: PathConfig)`

### Semantics
Renders a continuous, open polyline or closed area boundary of connected coordinate sequences. This acts as our general-purpose topology resolver for statistical charts and complex geographical data.

### Implementation
- **Vector backends (SVG, PDF)**: Continuous `<path>` or `<polyline>` string element.
- **CPU Raster backend (PNG)**: Iterative vertex stitching via `tiny_skia::PathBuilder` wrapped into a high-quality anti-aliased stroke sequence.
- **GPU Raster backend (WGPU)**: Pure GPU pipelines tailored by configuration. For statistical lines and areas, it bypasses CPU layout systems entirely using on-chip dynamic expansion. For complex geographical polygons with holes/islands, it performs an Ahead-Of-Time (AOT) lightweight triangulation via `earcutr` during data ingestion, streaming static triangle buffers directly to the GPU.

### Performance
- Bypasses traditional heavy runtime CPU tessellation engines (like `lyon`), replacing them with specialized hardware pathways or static AOT cache structures.
- Guarantees **zero CPU computing overhead during web interaction (pan/zoom)**.

### Use Cases
- Continuous line marks and density curves
- Area plots and stacked ribbons
- Geographic maps and multi-layered complex boundaries

---

## 6. `draw_text(&mut self, config: TextConfig)`

### Semantics
Renders formatted text with strict layout attributes matching SVG's `text-anchor` and `dominant-baseline` rules.

### Implementation
- **Vector backends (SVG, PDF)**: Native `<text>` with text layout attributes.
- **CPU Raster backend (PNG)**: Direct horizontal width tracking and baseline offset calculations powered by `ab_glyph`, manually extracting and drawing outlines via `tiny_skia::PathBuilder` curves.
- **GPU Raster backend (WGPU)**: Cohesive native integration with `glyphon`, compositing character atlases directly onto the target pass texture.

### Performance
- Eliminates intermediate browser canvas allocation and heavy WebAssembly-to-JS pixel memory copy operations.

### Use Cases
- Data labels, axis labels, titles, and legends

---

## 7. `draw_gradient_rect(&mut self, config: GradientRectConfig)`

### Semantics
Fills an axis-aligned rectangle with a seamless linear gradient.

### Implementation
- **Vector backends (SVG, PDF)**: Context-linked `<linearGradient>` reference fields.
- **CPU Raster backend (PNG)**: Evaluated directly via `tiny_skia::LinearGradient` set with custom `GradientStop` vectors and absolute coordinate spans.
- **GPU Raster backend (WGPU)**: Custom fragment shader computing interpolations directly on instanced quads.

---

# Mark-to-Primitive Routing Rules

The real power of this design is how high-level marks map to low-level primitives. This routing is deterministic, optimized, and identical across all backends. Notice how statistical lines and area bounds bypass heavy vector path layout constraints to utilize streamlined hardware pathways:

| Mark Type | Primary Primitive Mapping | Backend Execution Strategy |
| :--- | :--- | :--- |
| **PointMark** | `draw_circle`, `draw_rect`, `draw_polygon` | SVG Native / CPU Vector Paths / GPU Instanced SDF & Templates |
| **LineMark** | `draw_path` (Polyline context) | SVG Polyline / Skia Antialiased Stroke / GPU On-Chip Line Extrusion |
| **BoxplotMark**| `draw_rect`, `draw_line`, `draw_circle` | Decomposed cleanly into atomic backend shape calls |
| **TextMark** | `draw_text` | SVG Typography / Skia Glyph Path Extraction / GPU Glyphon Pass |
| **AreaMark** | `draw_path` (Closed context) | SVG Path / Skia Monotonic Scanlines / GPU Linear Triangulation Loops |
| **BarMark** | `draw_rect` | Highly optimized direct bounding-box rectangle fills |
| **Heatmap** | `draw_gradient_rect` | SVG Gradients / Skia Structural Shaders / GPU Interpolation Shaders |
| **Geographic** | `draw_path` (Complex context) | SVG Unified Paths / Skia Closed Geometry / GPU AOT `earcutr` Index Cache |

---

# WGPU Implementation Notes (For Backend Developers)

The WGPU backend strictly adheres to the pure hardware acceleration contract defined above.

The `chart.wgsl` shader implements specialized pipelines, each corresponding to an optimized primitive:

1. **Circle Pipeline**: Instanced SDF rendering on a single quad.
2. **Line Pipeline**: Instanced line segments and polylines expanded dynamically on-chip using WGSL vertex shaders.
3. **Path Pipeline**: GPU-native rendering of static triangle streams indexed via AOT geospatial partitioning or simple area loops.
4. **Rect Pipeline**: Instanced quads with fragment shader clipping for rounded corners.
5. **Polygon Pipeline**: Instanced rendering of CPU-precomputed regular polygon vertices.
6. **Gradient Rect Pipeline**: Instanced quads with linear gradient fragment shader.

Text rendering is natively handled by the integrated `glyphon` text runner, overlaying labels directly within the pass layout without relying on expensive CPU-to-GPU memory roundtrips.