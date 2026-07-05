# Integrating Charton with Bevy (Decoupled Architecture)

When attempting to embed charts into a powerful game engine like Bevy, you often run into the brutal reality of the Rust ecosystem: WGPU Version Hell. If the `wgpu` version required by Bevy mismatches the one used by your charting library, the compiler's type system will flat-out reject your build.

Charton's `WgpuRenderer` provides an elegant Decoupled Rendering solution to bypass this entirely. It spins up its own independent GPU context in the background, renders the chart off-screen, reads the pixels back to CPU memory (`Vec<u8>`), and then uploads them to Bevy as a standard texture. This design allows you to completely ignore dependency conflicts between the two ecosystems.

Here is the complete integration logic and a step-by-step breakdown of the source code.

## Cargo Dependencies

In your host Bevy project, you only need the core `bevy` library and `charton` with the `wgpu` and `png` feature enabled. Because the rendering is completely decoupled, you do not need to manually import `wgpu` or `pollster` here.

```toml
[package]
name = "bevy"
version = "0.1.0"
edition = "2024"

[dependencies]
charton = { version = "0.5", features = ["wgpu", "png"] }
bevy = "0.18"
```

## Step-by-Step Architecture Breakdown

The core of this integration is "separation of concerns." We break the entire lifecycle down into four clear steps:

### Step 1: Initialize the Host App (App Config)

Configure the base Bevy plugins. Because modern Bevy strongly types `WindowResolution` to avoid floating-point precision issues, we must pass an explicit integer tuple `(800, 600)`. We then mount our core rendering logic into the `Startup` schedule, ensuring it only runs once.

### Step 2: Headless Chart Generation (Phase 1: Charton Domain)

The `WgpuRenderer` initializes silently in the background. It doesn't rely on any visible window; instead, it allocates an off-screen canvas in VRAM, draws the chart primitives, and overlays text via the CPU. Finally, calling `.render()` yields a pure, raw `Vec<u8>` array of RGBA pixels.

### Step 3: The Asset Handshake (Phase 2: Memory Bridge)

Once we have the pixels on the CPU side, we wrap them in a Bevy Image struct. We explicitly define the texture format as `TextureFormat::Rgba8Unorm` to match Charton's output. By calling `images.add(chart_image)`, we register this raw memory block into Bevy's asset pipeline and receive a lightweight pointer: `Handle<Image>`.

### Step 4: Modern Component Spawning (Phase 3: Bevy Domain)

Thanks to modern Bevy's Required Components feature, we can say goodbye to bloated `Bundles`. By simply spawning the core `Camera2d` and `Sprite::from_image(texture_handle)` components, Bevy's underlying engine automatically detects dependencies and injects `Transform`, `Visibility`, and other required components to perfectly center the chart on screen.

## The Complete Implementation

Here is the complete, standalone rendering example that compiles and runs flawlessly in a Bevy 0.18 environment:

```rust
use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use charton::prelude::*;
use charton::render::WgpuRenderer;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Charton Bevy 0.18 Decoupled Example".into(),
                resolution: (800, 600).into(), // Strictly requires a (u32, u32) integer tuple
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    // 1. Initialize 2D Camera (Camera2dBundle is deprecated in modern Bevy)
    commands.spawn(Camera2d);

    // ==========================================
    // Phase 1: Chart Rendering (Headless & Decoupled)
    // ==========================================
    // Spin up an independent GPU instance for off-screen rendering
    let mut charton_renderer = WgpuRenderer::new();

    let ds = load_dataset("mtcars").unwrap();
    let chart = Chart::build(&ds)
        .unwrap()
        .mark_point()
        .unwrap()
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear").with_scale(Scale::Discrete),
        ))
        .unwrap()
        .configure_theme(|t| t.with_label_size(20.0).with_tick_label_size(18.0))
        .with_size(800, 600);

    // The perfect black box: compress complex chart rendering into a pure byte array
    let pixels = charton_renderer.render(&chart, 800, 600, 1.0).unwrap();

    // ==========================================
    // Phase 2: Bevy 0.18 Integration 
    // ==========================================
    let size = Extent3d {
        width: 800,
        height: 600,
        depth_or_array_layers: 1,
    };

    // Construct an Image asset recognized by Bevy
    let chart_image = Image::new(
        size,
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8Unorm, 
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    // Store the image into Bevy's asset pipeline and acquire the unique handle
    let texture_handle = images.add(chart_image);

    // 2. Render the Sprite (SpriteBundle is deprecated; Required Components handles the rest)
    commands.spawn(Sprite::from_image(texture_handle));
}
```

## Architectural Takeaways

**Performance Strategy: The One-Shot Rule**

Placing `WgpuRenderer` inside Bevy's `Startup` system is critical to this architecture. Memory readback (transferring data from GPU to CPU, then back to GPU) is an incredibly expensive operation. If you drop this into the `Update` schedule—which runs 60 times a second—you will block the main thread and tank your framerate. Treating the chart as a static asset, generating it once, and caching it is the best practice for maintaining game-level performance.

Through this approach, it doesn't matter how fast the Bevy community iterates its rendering pipeline, nor does it matter how many major versions separate the `wgpu` dependencies. As long as `Vec<u8>` remains a universal memory bridge, Charton can seamlessly integrate into any complex Bevy project with minimal effort and maximum stability.