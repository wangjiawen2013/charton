# Rendering Primitives & Backend Contract

This chapter defines the unified, internal rendering contract that powers all Charton backends (SVG, PNG, PDF, WGPU). These primitives are not exposed to end users—they form the low-level geometry layer that translates declarative marks (Point, Line, Boxplot, Text, Area) into pixels or vectors.

The design follows strict principles of semantic clarity, cross-backend consistency, performance optimization, and implementation simplicity. Every method has a single, well-defined responsibility.

---

# Core Design Principles

Charton’s RenderBackend is engineered around five non-negotiable constraints:

## Backend Agnosticism

A single implementation of marks (Point, Line, Boxplot, Area) must work identically across vector (SVG, PDF) and raster (PNG, WGPU) targets without modification. The same input produces identical visual output regardless of backend.

## Semantic Separation

Each drawing method corresponds to a distinct geometric category. No method tries to represent multiple visual concepts. This eliminates ambiguity for both backend implementors and mark authors.

## Cross-Backend Semantic Consistency

All backends must implement exactly the same set of primitives with identical semantics. What `draw_polygon` means in SVG is exactly what it means in WGPU. This is the foundation of Charton's reliability.

## Performance by Design

Primitives are optimized for their intended use case: instanced SDF for circles, instanced quads with fragment clipping for rectangles, CPU precomputed vertices for symmetric markers, tessellation for complex paths, simple lines for some visual components (such as `errorbar`, `rule`), and native text for labels.

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

- Vector backends (SVG, PDF): Native circle primitive.
- Raster backends (PNG, WGPU): SDF (signed distance field) shader on a single quad.

### Performance

- Extremely fast: constant-time shader math, no tessellation, no vertex generation.
- Ideal for millions of scatter plot markers.

### Use Cases

- Point marks (circular style)
- Outlier points in boxplots
- Circular markers and dots

This primitive exists because circles cannot be perfectly represented by polygons without high vertex counts. It is both faster and sharper than emulating circles via `draw_polygon`.

---

## 2. `draw_rect(&mut self, config: RectConfig)`

### Semantics

Draws an axis-aligned rectangle from position, width, and height, with optional rounded corners.

### Implementation

- Vector backends (SVG, PDF): Native rectangle primitive with `rx`/`ry` attributes.
- Raster backends (PNG, WGPU): Instanced quad with fragment shader clipping for rounded corners.

### Performance

- Near-optimal performance: simple bounds, no complex geometry.
- Supports millions of instances for heatmaps and bar charts.

### Use Cases

- Boxplot boxes
- Bar marks
- Axis rectangles
- Heatmap cells
- UI panels and backgrounds

Rectangles are ubiquitous in charts. A dedicated primitive avoids redundant vertex generation and enables backend-level optimizations.

---

## 3. `draw_polygon(&mut self, config: PolygonConfig)`

### Semantics

Renders a closed, convex, regular polygon used exclusively for symmetric visual markers.

### Implementation

- Vector backends (SVG, PDF): Closed path generated from regular polygon vertices.
- Raster backends (PNG, WGPU): CPU-precomputed vertices uploaded directly to GPU as instanced geometry.

### Performance

- Fast for symmetric marker shapes: vertex generation happens once at startup, not per frame.
- Perfect for thousands to millions of non-circular point markers.
- Not intended for arbitrary or concave polygons—those belong to `draw_path`.

### Use Cases

- Point markers: Triangle, Diamond, Pentagon, Hexagon, Star, Octagon
- Symmetric small shapes in legends and annotations

This method is semantically distinct from `draw_path`. It exists solely for regular, convex markers that are common in data visualization.

---

## 4. `draw_line(&mut self, config: LineConfig)`

### Semantics

Draws a single, straight line segment between two explicit points.

### Implementation

- All backends: Native line primitive.
- WGPU: Expanded to a quad in the vertex shader for consistent line width across all scales.

### Performance

- Minimal overhead: no path expansion, no grouping, no tessellation.
- Ideal for thousands of discrete line segments like grid lines and ticks.

### Use Cases

- Boxplot whiskers
- Median lines
- Axis ticks
- Simple grid strokes
- Error bars

This primitive exists for discrete, non-connected lines. It is lighter than `draw_path` and expresses clear intent.

---

## 5. `draw_path(&mut self, config: PathConfig)`

### Semantics

Renders a continuous, open or closed path of connected line segments or curves. Supports arbitrary shapes, holes, and complex geometry.

### Implementation

- Vector backends (SVG, PDF): Continuous path element.
- Raster backends (PNG, WGPU): Lyon tessellation into triangles.

### Performance

- Supports polyline, area, and complex geometry.
- Used only where shape cannot be represented by simpler primitives.
- Higher overhead than specialized primitives due to tessellation requirements.

### Use Cases

- Line marks
- Area plots
- Density curves
- Concave polygons
- Geographic shapes
- Multi-segment connected geometry
- Shapes with holes

This is Charton’s general-purpose geometry primitive for complex visualizations. It should never be used for shapes that can be represented by `draw_circle`, `draw_rect`, or `draw_polygon`.

---

## 6. `draw_text(&mut self, config: TextConfig)`

### Semantics

Renders formatted text with alignment, baseline, weight, and rotation.

### Implementation

- Vector backends (SVG, PDF): Native text.
- Raster backends (PNG, WGPU): Glyph atlas + textured quads.

### Performance

- Independent pipeline; does not interfere with shape rendering.
- Cached glyphs for repeated text elements.

### Use Cases

- Data labels
- Axis labels
- Text marks
- Annotations
- Titles and legends

Text requires layout, font management, and alignment logic that cannot be modeled by shape primitives.

---

## 7. `draw_gradient_rect(&mut self, config: GradientRectConfig)`

### Semantics

Fills a rectangle with a linear gradient.

### Implementation

- Vector backends (SVG, PDF): Defined gradient elements.
- Raster backends (PNG, WGPU): Custom shader with angle support.

### Performance

- Specialized for heatmaps and gradient backgrounds.
- No texture uploads required for simple linear gradients.

### Use Cases

- Heatmap cells
- Gradient backgrounds
- Themed panels
- Color bars and legends

This primitive enables publication-ready visual styles without exposing complex backend APIs.

---

# Critical Design Decision: Independent `draw_polygon` Pipeline

## The Rationale for Separation

After extensive evaluation of alternative designs (including merging polygons into the path pipeline), we have retained an independent `draw_polygon` primitive and dedicated pipeline across all backends. This decision is based on three foundational principles:

### 1. Semantic Clarity & Intent Preservation

`draw_polygon` has a single, well-defined purpose: rendering regular, convex markers for point plots.

`draw_path` has a different purpose: rendering arbitrary, complex, continuous geometry.

Merging them would destroy semantic distinction. A mark author would have no way to know if their triangle marker is being rendered as an optimized instance or as a tessellated path.

This separation eliminates an entire class of bugs where backend implementors might interpret "polygon" differently.

### 2. Cross-Backend Consistency

SVG and PNG backends already have mature, production-proven implementations of `draw_polygon` as a separate primitive.

Changing the contract now would require breaking changes to these stable backends, which is unacceptable.

By retaining the same interface across all backends, we ensure that charts look identical whether rendered to SVG, PNG, or WGPU.

This consistency is critical for Charton's core value proposition:

> "write once, render anywhere"

### 3. Engineering & Performance Advantages

#### CPU Precomputation

Regular polygon vertices can be computed once at library initialization, not per frame or per instance.

#### Instanced Rendering

WGPU can render millions of identical polygon markers in a single draw call with minimal GPU memory usage.

#### No Tessellation Overhead

Unlike arbitrary paths, regular polygons never require runtime tessellation.

#### Simpler Backend Implementation

Each backend can implement `draw_polygon` in the most natural way for its architecture without compromising on performance.

#### Clear Optimization Boundaries

We can optimize the polygon pipeline independently of the path pipeline, which is critical for scatter plot performance.

---

## Why Not SDF for Polygons?

While SDF (Signed Distance Field) shaders are excellent for circles, they are not the optimal solution for all polygon types:

- SDF for complex shapes (like stars) requires more complex math and higher shader execution time.
- SDF anti-aliasing can produce inconsistent results across different polygon types.
- Most importantly, SDF would break semantic consistency with SVG and PNG backends, which render polygons as vertex-based shapes.

By using CPU-precomputed vertices for polygons, we achieve:

- Perfect visual consistency across all backends
- Simpler shader code
- Better performance for most common marker types
- Easier extensibility for new marker shapes

---

# Mark-to-Primitive Routing Rules

The real power of this design is how high-level marks map to low-level primitives. This routing is deterministic, optimized, and identical across all backends:

| Mark Type | Primitive Mapping |
|------------|------------------|
| PointMark | `draw_circle`, `draw_rect`, `draw_polygon` |
| LineMark | `draw_path` |
| BoxplotMark | `draw_rect`, `draw_line`, `draw_circle` |
| TextMark | `draw_text` |
| AreaMark | `draw_path` |
| BarMark | `draw_rect` |
| Heatmap | `draw_gradient_rect` |
| LegendMark | `draw_circle`, `draw_rect`, `draw_polygon`, `draw_text` |

---

# Summary

Charton’s RenderBackend is a masterfully minimal interface that balances:

- Semantic expressiveness: Each primitive represents exactly one visual concept
- Cross-backend consistency: Identical behavior across SVG, PNG, PDF, and WGPU
- Performance at scale: Specialized pipelines for the most common visualization patterns
- Ease of implementation: Clear boundaries between primitives simplify backend development
- Backward compatibility: No breaking changes to existing stable backends

The independent `draw_polygon` pipeline is not a compromise—it is a deliberate design choice that strengthens Charton's position as a reliable, high-performance visualization library.

Every method exists for a reason.

Every choice serves the grammar of graphics, the Rust ownership model, and the demands of production visualization.

---

# WGPU Implementation Notes (For Backend Developers)

The WGPU backend strictly adheres to the contract defined above.

The `chart.wgsl` shader implements six independent pipelines, each corresponding to exactly one primitive:

1. Circle Pipeline: Instanced SDF rendering on a single quad
2. Line Pipeline: Instanced line segments expanded to quads in the vertex shader
3. Path Pipeline: Tessellated triangles for arbitrary geometry
4. Rect Pipeline: Instanced quads with fragment shader clipping for rounded corners
5. Polygon Pipeline: Instanced rendering of CPU-precomputed regular polygon vertices
6. Gradient Rect Pipeline: Instanced quads with linear gradient fragment shader

All pipelines share the same uniform buffer and bind group layout, ensuring efficient resource management.

Text rendering is handled by a separate text renderer that is not part of the core shader.