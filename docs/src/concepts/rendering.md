# Rendering Primitives & Backend Contract

This chapter defines the unified, internal rendering contract that powers all Charton backends (SVG, PNG, PDF, WGPU). These primitives are not exposed to end users—they form the low-level geometry layer that translates declarative marks (Point, Line, Boxplot, Text, Area) into pixels or vectors.

The design follows strict principles of semantic clarity, cross-backend consistency, performance optimization, and implementation simplicity. Every method has a single, well-defined responsibility.

## Core Design Principles

Charton’s `RenderBackend` is engineered around four non-negotiable constraints:

### Backend Agnosticism
A single implementation of marks (Point, Line, Boxplot, Area) must work identically across vector (SVG, PDF) and raster (PNG, WGPU) targets without modification.

### Semantic Separation
Each drawing method corresponds to a distinct geometric category. No method tries to represent multiple visual concepts.

### Performance by Design
Primitives are optimized for their intended use case: instanced SDF for markers, tessellation for complex paths, simple lines for axis components, and native text for labels.

### Minimalism & Completeness
The interface provides only the necessary primitives to build every chart in the grammar—no bloat, no missing building blocks.

## The RenderBackend Primitives: Semantics & Usage

Below is the formal definition of each primitive, its semantic role, performance characteristics, and intended use cases across all backends.

---

### 1. `draw_circle(&mut self, config: CircleConfig)`

**Semantics**: Renders a perfect circle defined by a center point and radius.

**Implementation**:
- Vector backends: Native circle primitive.
- Raster/WGPU: SDF (signed distance field) shader on a single quad.

**Performance**:
- Extremely fast: constant-time shader math, no tessellation.
- Ideal for millions of markers.

**Use Cases**:
- Point marks
- Outlier points in boxplots
- Circular markers and dots

This primitive exists because circles cannot be perfectly represented by polygons without high vertex counts. It is both faster and sharper than emulating circles via `draw_polygon`.

---

### 2. `draw_rect(&mut self, config: RectConfig)`

**Semantics**: Draws an axis-aligned rectangle from position, width, and height.

**Implementation**:
- Vector backends: Native rectangle.
- WGPU: SDF or instanced quad.

**Performance**:
- Near-optimal performance: simple bounds, no complex geometry.

**Use Cases**:
- Boxplot boxes
- Bar marks
- Axis rectangles
- Heatmap cells

Rectangles are ubiquitous in charts. A dedicated primitive avoids redundant vertex generation and enables backend-level optimizations.

---

### 3. `draw_polygon(&mut self, config: PolygonConfig)`

**Semantics**: Renders a closed, convex, regular polygon used exclusively for markers.

**Implementation**:
- Vector backends: Closed path from points.
- WGPU: SDF for regular shapes (triangle, hexagon, diamond).

**Performance**:
- Fast for symmetric marker shapes.
- Not intended for arbitrary or concave polygons.

**Use Cases**:
- Point markers: Triangle, Diamond, Pentagon, Hexagon, Star
- Symmetric small shapes

This method is reserved for visual markers, not geographic or complex shapes. It is semantically distinct from `draw_path`.

---

### 4. `draw_line(&mut self, config: LineConfig)`

**Semantics**: Draws a single, straight line segment between two explicit points.

**Implementation**:
- All backends: Native line primitive.

**Performance**:
- Minimal overhead: no path expansion, no grouping.

**Use Cases**:
- Boxplot whiskers
- Median lines
- Axis ticks
- Simple grid strokes

This primitive exists for discrete, non-connected lines. It is lighter than `draw_path` and expresses clear intent.

---

### 5. `draw_path(&mut self, config: PathConfig)`

**Semantics**: Renders a continuous, open or closed path of connected line segments or curves.

**Implementation**:
- Vector backends: Continuous path.
- WGPU: Lyon tessellation into triangles.

**Performance**:
- Supports polyline, area, and complex geometry.
- Used where shape cannot be represented by simpler primitives.

**Use Cases**:
- Line marks
- Area plots
- Density curves
- Concave polygons
- Geographic shapes
- Multi-segment connected geometry

This is Charton’s general-purpose geometry primitive for complex visualizations.

---

### 6. `draw_text(&mut self, config: TextConfig)`

**Semantics**: Renders formatted text with alignment, baseline, weight, and rotation.

**Implementation**:
- Vector backends: Native text.
- WGPU: Glyph atlas + textured quads.

**Performance**:
- Independent pipeline; does not interfere with shape rendering.

**Use Cases**:
- Data labels
- Axis labels
- Text marks
- Annotations

Text requires layout, font management, and alignment logic that cannot be modeled by shape primitives.

---

### 7. `draw_gradient_rect(&mut self, config: GradientRectConfig)`

**Semantics**: Fills a rectangle with a linear gradient.

**Implementation**:
- Vector backends: Defined gradient elements.
- Raster/WGPU: Custom shader or gradient texture.

**Performance**:
- Specialized for heatmaps and gradient backgrounds.

**Use Cases**:
- Heatmap cells
- Gradient backgrounds
- Themed panels

This primitive enables publication-ready visual styles without exposing complex backend APIs.

---

## Why These Exact Primitives?

### Semantic Clarity
Each method represents exactly one visual concept:
- Markers: `draw_circle`, `draw_rect`, `draw_polygon`
- Simple lines: `draw_line`
- Complex paths: `draw_path`
- Text: `draw_text`
- Stylized rectangles: `draw_gradient_rect`

No overlap, no ambiguity, no guesswork for backend implementors.

### Performance Engineering
- Marks use SDF → millions of points at high FPS
- Lines use lightweight segments → minimal CPU overhead
- Complex geometry uses tessellation → correctness for areas and curves
- Text uses independent pipeline → no performance interference

### Cross-Backend Stability
All four backends (SVG, PDF, PNG, WGPU) share the exact same contract. Charts look identical regardless of output target. This is the foundation of Charton’s reliability.

## Mark-to-Primitive Routing Rules

The real power of this design is how high-level marks map to low-level primitives:

- PointMark → `draw_circle`, `draw_rect`, `draw_polygon`
- LineMark → `draw_path`
- BoxplotMark → `draw_rect`, `draw_line`, `draw_circle`
- TextMark → `draw_text`
- AreaMark → `draw_path`
- BarMark → `draw_rect`
- Heatmap → `draw_gradient_rect`

This routing is deterministic, optimized, and maintainable.

## Summary

Charton’s `RenderBackend` is a masterfully minimal interface that balances:
- Semantic expressiveness
- Cross-backend consistency
- Performance at scale
- Ease of implementation

Every method exists for a reason. Every choice serves the grammar of graphics, the Rust ownership model, and the demands of production visualization.