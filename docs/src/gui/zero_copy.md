# Zero-Copy Rendering (Shared WGPU Context)

The most performant way to integrate Charton into an existing application is to inject your app's existing `wgpu::Device` and `wgpu::Queue` directly into Charton's rendering pipeline.

By using `render_to_surface`, Charton never creates its own GPU context. Instead, it generates WGPU commands and executes them directly onto a `TextureView` provided by your application's render loop. Pixels never leave the GPU.

## The Integration Flow

Initialize: Wrap your app's `wgpu::Device` and `Queue` in Charton's `WgpuBackend`.

Provide Target: Give Charton a `TextureView` (often an offscreen texture or the swapchain surface).

Render: Await `chart.render_to_surface(...)`.

## Example: High-Performance Render Loop

While the exact implementation depends on your framework (e.g., Bevy's render graph or egui's custom 3D callbacks), the core logic looks like this inside your frame update:

```rust
use charton::prelude::*;
use charton::render::WgpuBackend;

// 1. Build your declarative chart (typically done once or when data changes)
let chart = Chart::build(&dataset)
    .unwrap()
    .mark_line()
    .unwrap()
    .encode((alt::x("time"), alt::y("value")))
    .unwrap()
    .with_size(800, 600);

// --- Inside your app's render loop / frame callback ---

// `app_device`, `app_queue`, and `target_view` are provided by your GUI framework.
async fn render_frame(
    app_device: wgpu::Device, 
    app_queue: wgpu::Queue, 
    target_view: &wgpu::TextureView
) {
    // 2. Wrap the host's GPU context in Charton's backend.
    // This wrapper is lightweight and should be reused if possible.
    let mut charton_backend = WgpuBackend::new(
        app_device.clone(),
        app_queue.clone(),
        800,   // width
        600,   // height
        1.0,   // scale_factor
    ).await;

    // 3. Inject the external texture view into Charton
    // Charton writes geometry directly to your texture without touching the CPU.
    let _text_ledger = chart
        .render_to_surface(&mut charton_backend, target_view)
        .await
        .expect("Charton failed to render to surface");
    
    // Note: The returned `_text_ledger` contains text rendering configs.
    // For full zero-copy, you would pass this ledger to your UI framework's 
    // text renderer (e.g., egui's painter) to composite the text on top.
}
```

> *Important Constraint: To use `render_to_surface`, your application's `Cargo.toml` must depend on the exact same version of `wgpu` that Charton uses. If they mismatch, Rust's type system will reject the `wgpu::Device` injection.*