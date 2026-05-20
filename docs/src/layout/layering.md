# Layering Grammar

A single geometric mark can rarely tell a complete data story. To compare raw observations against a mathematical baseline, or to display standard error bars over a bar chart, we need the ability to combine multiple visual elements. In the Grammar of Graphics, this is achieved through **Layering**.

Layering allows you to stack multiple independent plots on top of each other within a shared spatial and mathematical plane, creating sophisticated, multi-valued visualizations.

## The Layer as an Independent Declarative Unit

In the system architecture, a layer is encapsulated as an independent specification. Each layer maintains its own isolated context:
* **Isolated Dataset**: A specific Polars `DataFrame`.
* **Local Encodings**: Individual specifications mapping its data fields to visual channels (e.g., Layer A maps `X` and `Y`, while Layer B maps `X`, `Y`, and `Color`).
* **Geometric Mark**: The distinct renderer (e.g., Point, Line, Bar) that dictates its final drawing primitive.

When you compose a layered chart, you do not perform database joins or manual array concatenations. Instead, you build an ordered vector of these independent logical units:

```rust
// Composing a layered chart by stacking a Line trend over a Point scatter
let composite_chart = LayeredChart::new()
    .add_layer(Chart::new(df_points).mark_point().encode((x("age"), y("height"))))
    .add_layer(Chart::new(df_trend).mark_line().encode((x("age"), y("predicted_height"))));
```

## Shared Context vs. Local Overrides

A fundamental challenge in multi-layer rendering is balancing global consistency with local flexibility. The orchestrator divides constraints into two categories:

### Shared Global Constraints

By default, all layers in a composite chart share a singular Coordinate System (e.g., a shared Cartesian plane) and a unified set of Aesthetic Scales. If both a scatter layer and a line layer map a categorical field named `"species"` to the `Color` channel, they are strictly bound to the exact same color palette.

### Local Scale Preferences

Despite sharing the same axis, an individual layer can request distinct structural parameters. For example, Layer A might explicitly request a `Linear` scale transformation, while Layer B requests a `Log` scale for its specific value range.

The global orchestrator evaluates these conflicting requests during the initialization phase, using deterministic hierarchy rules to resolve them into a single mathematical source of truth.

## State Injection & Interior Mutability

Because layers are entirely decoupled when declared by the user, the engine must bridge the gap between abstract global scales and localized rendering. This is achieved via a dedicated Injection Phase utilizing Rust's interior mutability patterns.

1. The Shared Pointer Channel: The global coordinate system and aesthetic mappings are compiled into thread-safe shared pointers (`Arc<dyn CoordinateTrait>` and `GlobalAesthetics`).
2. Back-Filling via Interior Mutability: The orchestrator traverses the layer vector and invokes .`inject_resolved_scales(...)`.
3. Caching the Scales: Because layers are passed around by shared reference or standard borrowing during the high-level coordination phase, they internally utilize mutation safe-guards (such as `RwLock` or `OnceLock`) to cache these global scales.

Once back-filling is complete, every layer possesses immediate, zero-copy access to the final canvas boundaries and visual mappers.

## The Realization Pipeline & Render Order

When the chart transitions to physical drawing, execution follows a strict sequential pipeline within a localized `PanelContext`:

* Z-Index Execution: The engine iterates through the resolved layers in the exact order they were registered by the user (First In, First Out). The first layer added forms the background foundation, while subsequent layers are painted directly on top.
* Stateless Mark Rendering: During this phase, individual geometric marks are entirely stateless. They do not know—nor do they need to know—if they are rendering alone or alongside ten other layers. They simply read their local data arrays, map them through the injected global coordinate scales, and emit concrete primitives (Circles, Paths, Rects) to the active `RenderBackend`.

## Automated Legend Unification

Layering does not just unify spatial axes; it also unifies visual guides. If multiple layers share an identical aesthetic mapping (e.g., both a Point mark and an Area mark map the color channel to a field named `"group"`):

* Semantic Deduction: The guide engine detects the shared field name across the global aesthetic registry.
* Consolidation: Instead of rendering two separate legends that compete for canvas real estate, the system merges the structural definitions into a single, comprehensive Legend Spec.
* Space Preservation: This consolidated specification is handed to the layout manager, ensuring that margins are calculated once, maximizing the physical pixels left for the actual plot panel.

## Key Takeaways

* Layers are logical stacks: They are isolated in specification but unified in mathematical space.
* Interior mutability allows the global orchestrator to inject resolved scales into read-only layer definitions without expensive object copying.
* Sequential rendering enforces a predictable Z-index layout, allowing complex visualizations to be built out of simple, composable geometric blocks.