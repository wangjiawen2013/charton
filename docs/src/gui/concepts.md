# GUI Integration Concepts

Integrating a data visualization library into a native desktop application or game engine introduces a unique challenge: who owns the GPU context?

Most native frameworks (like `Bevy`, `egui`, `Iced`, or raw `winit`) create and manage their own WGPU instance, device, and render loop. If your charting library also tries to manage the GPU, you run into resource conflicts, thread blocking, or dependency hell (mismatched wgpu versions).

Charton solves this by treating rendering as a purely functional pipeline. We offer two distinct integration strategies depending on your application's needs:

## The Two Paradigms

| Feature | Zero-Copy Rendering (render_to_surface) | Decoupled Rendering (WgpuRenderer) |
|------|------------------------------------------|-------------------------------------|
| Data Flow | GPU Only (Zero-Copy) | GPU → CPU → GPU |
| Performance | Maximum (Native 60+ FPS) | Moderate (Memory readback overhead) |
| WGPU Context | Shared with Host App | Charton creates its own |
| Dependency | Host and Charton wgpu versions must match | Version independent (Black-box) |
| Best For | Game engines, fluid dashboards, real-time data | Background workers, CLI tools, basic GUI |

If your application demands smooth, interactive 60 FPS animations, choose Zero-Copy. If you are building a tool where charts update infrequently and you want to completely avoid dependency conflicts, choose Decoupled.

The following chapters will walk you through implementing both approaches.