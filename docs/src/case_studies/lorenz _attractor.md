# The 100k-Point Lorenz Attractor (WGPU & Zero-Allocation WASM)
While the previous chapter demonstrated how to hook up charton to WGPU, our implementation was essentially a "naive" port. We were still rebuilding the Dataset and re-allocating memory on the heap every single frame.

To truly push WebAssembly and your graphics card to their absolute limits, we need to talk about Chaos Theory, glowing butterflies, and Stateful Memory Management.

In this case study, we will render a Lorenz Attractor—a complex, non-repeating 3D mathematical trajectory. We will simulate and project 100,000 dynamic particles rotating in 3D space, pushing the results to WGPU at a locked 60 FPS.

More importantly, we will refactor our WASM boundary to achieve Zero-Allocation (Zero Malloc) during the render loop.

## The Bottleneck: Why "Naive" WASM Stutters
If you tried to push the code from Part 2 to 100,000 points, you might have noticed occasional micro-stutters. The culprit isn't the GPU; it's the CPU struggling with memory allocation.

In our previous `render_chart_gpu` function, we executed this every frame:

```rust
let ds = Dataset::new()
    .with_column("x", xs.to_vec()) // Heap Allocation!
    // ...
```

At 60 FPS with 100k points, `to_vec()` forces the WASM memory allocator to find, reserve, copy, and free megabytes of memory hundreds of times a second. The Garbage Collector/Allocator chokes, and the GPU is left starving for data.

To solve this, we must shift from a Stateless API (calling a function every frame) to a Stateful Architecture (instantiating a persistent Rust struct that reuses memory capacity).

## Rust: The Stateful LiveChartApp
We will create a long-lived Rust object that pre-allocates the memory for our 100,000 points once during startup. During the 60 FPS animation loop, we will use Charton's high-performance update_column_f64 method to perform an in-place memcpy, completely bypassing heap allocation.

Update your src/lib.rs with the following advanced pattern:

```rust
use wasm_bindgen::prelude::*;
use charton::prelude::*;
use charton::scale::ScaleDomain;

/// The stateful application context for high-performance rendering.
/// By holding the `Dataset` in memory, we avoid the overhead of re-allocating
/// data structures during every frame of the 60 FPS render loop.
#[wasm_bindgen]
pub struct LiveChartApp {
    dataset: Dataset,
    canvas_id: String,
}

#[wasm_bindgen]
impl LiveChartApp {
    /// 1. Bootstrapping Phase: Runs exactly once.
    /// Pre-allocates memory for the entire dataset so the render loop never has to.
    /// This is crucial for avoiding Garbage Collection (GC) jank in WebAssembly.
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: String, capacity: usize) -> Result<LiveChartApp, JsValue> {
        // Initialize with zeroed buffers of the exact capacity we need.
        // This reserves a contiguous block of heap memory upfront.
        let zeros = vec![0.0; capacity];
        
        let dataset = Dataset::new()
            .with_column("x", zeros.clone()).map_err(|e| e.to_string())?
            .with_column("y", zeros.clone()).map_err(|e| e.to_string())?
            .with_column("intensity", zeros).map_err(|e| e.to_string())?;

        Ok(Self { dataset, canvas_id })
    }

    /// 2. The Render Loop: Runs 60 times a second.
    /// Accepts raw slices from JavaScript and performs zero-allocation, 
    /// in-place capacity reuse using Charton's internal `update_column_f64`.
    pub async fn update_and_render(
        &mut self,
        xs: &[f64],
        ys: &[f64],
        colors: &[f64],
    ) -> Result<(), JsValue> {
        // IN-PLACE UPDATES: These methods overwrite the existing data buffers.
        // By bypassing the standard Arc clone penalty and utilizing `Arc::get_mut`,
        // we copy the new slices without dropping or re-allocating underlying heap capacity.
        self.dataset.update_column_f64("x", xs).map_err(|e| e.to_string())?;
        self.dataset.update_column_f64("y", ys).map_err(|e| e.to_string())?;
        self.dataset.update_column_f64("intensity", colors).map_err(|e| e.to_string())?;

        // Re-construct the declarative chart definition (which is extremely cheap) 
        // and flush the updated buffer to the canvas backend.
        Chart::build(self.dataset.clone())
            .map_err(|e| e.to_string())?
            .mark_point().map_err(|e| e.to_string())?
            .configure_point(|p| {
                // Use small, semi-transparent points for a glowing volumetric effect
                p.with_size(1.2).with_opacity(0.6)
            })
            .encode((
                // CRITICAL OPTIMIZATION: Lock the scale domains.
                // This prevents the CPU from performing expensive Min/Max scans 
                // on 50,000 points every single frame.
                alt::x("x").with_domain(ScaleDomain::Continuous(-40.0, 40.0)),
                alt::y("y").with_domain(ScaleDomain::Continuous(0.0, 50.0)),
                alt::color("intensity").with_domain(ScaleDomain::Continuous(0.0, 50.0)),
            ))
            .map_err(|e| e.to_string())?
            .with_size(800, 500)
            .configure_theme(|t| t.with_background_color("#0d1117"))
            .render_to_canvas(&self.canvas_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
```

## JavaScript: Chaos Math & 3D Projection
In `index.html`, we will calculate the Lorenz equations. To create the "rotating glowing butterfly" effect, we will compute the 3D coordinates statically once, and then apply a dynamic 3D-to-2D rotation matrix every frame before streaming the data into our WASM app.

Replace your script tag in `index.html` with this high-performance orchestrator:

```rust
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Charton WASM — Industrial Grade Streaming</title>
    <style>
        body { font-family: system-ui, sans-serif; display: flex; flex-direction: column; align-items: center; background: #0d1117; color: #c9d1d9; padding-top: 2rem; margin: 0; }
        #canvas-container { position: relative; width: 800px; height: 500px; border-radius: 12px; background: #161b22; border: 1px solid #30363d; box-shadow: 0 4px 16px rgba(0,0,0,0.6); overflow: hidden; }
        #chart-canvas { width: 800px; height: 500px; display: block; }
        .tag { margin-top: 1rem; color: #8b949e; font-size: 0.9rem; }
    </style>
</head>
<body>
    <h2>🦋 Charton: 50k Particles (Zero-Allocation Architecture)</h2>
    <div id="canvas-container">
        <canvas id="chart-canvas"></canvas>
    </div>
    <div class="tag">Active Particles: 50,000 | FPS: <span id="fps-counter" style="color: #58a6ff; font-weight: bold;">0</span></div>

    <script type="module">
        import init, { LiveChartApp } from './pkg/wave.js';

        async function run() {
            await init();

            const TOTAL_POINTS = 50_000;
            const fpsCounter = document.getElementById('fps-counter');

            // 1. Bootstrapping: Instantiate the stateful Rust App
            // This pre-allocates memory for 50,000 points inside the WebAssembly heap.
            const app = new LiveChartApp("chart-canvas", TOTAL_POINTS);

            // Pre-allocate JavaScript boundaries to avoid GC jank on the JS side.
            const xs = new Float64Array(TOTAL_POINTS);
            const ys = new Float64Array(TOTAL_POINTS);
            const colors = new Float64Array(TOTAL_POINTS);

            // Pre-compute 3D base points for the Lorenz Attractor
            const lx = new Float64Array(TOTAL_POINTS);
            const ly = new Float64Array(TOTAL_POINTS);
            const lz = new Float64Array(TOTAL_POINTS);

            let x = 0.1, y = 0.0, z = 0.0;
            const dt = 0.005;
            for (let i = 0; i < TOTAL_POINTS; i++) {
                x += 10.0 * (y - x) * dt;
                y += (x * (28.0 - z) - y) * dt;
                z += (x * y - (8.0 / 3.0) * z) * dt;
                lx[i] = x; ly[i] = y; lz[i] = z;
                colors[i] = z; 
            }

            let angle = 0;
            let lastTime = performance.now();

            // 2. The Render Loop
            async function frameLoop() {
                const now = performance.now();
                fpsCounter.textContent = Math.round(1000 / (now - lastTime));
                lastTime = now;

                angle += 0.01;
                const cosA = Math.cos(angle);
                const sinA = Math.sin(angle);

                // Update the JS Float64Arrays in-place
                for (let i = 0; i < TOTAL_POINTS; i++) {
                    xs[i] = lx[i] * cosA - ly[i] * sinA;
                    ys[i] = lz[i]; 
                }

                // Stream arrays directly into the Rust app's pre-allocated memory
                try {
                    await app.update_and_render(xs, ys, colors);
                } catch (e) {
                    console.error("Render failed:", e);
                }

                requestAnimationFrame(frameLoop);
            }
            frameLoop();
        }
        run();
    </script>
</body>
</html>
```

## The Result: High-Density Chaos
When you run this code, you will see something mesmerizing: 100,000 points swirling in a perfectly fluid, anti-aliased 3D rotation. Because the points are small (1.2) and semi-transparent (0.6), they visually accumulate where the math dictates higher density, causing the "wings" of the butterfly to physically glow.

Because we eliminated the heap allocations, your CPU utilization will drop significantly, allowing the requestAnimationFrame loop to run completely unhindered, feeding WGPU exactly as fast as your monitor can refresh.

## Summary: Rules for High-Performance WASM
If you are building data-intensive applications (like live sensor dashboards or complex animations) that cross the JavaScript/WASM boundary, keep these golden rules in mind:

1. State is King: Never use functional, stateless APIs for high-frequency loops. Instantiate a Rust struct (#[wasm_bindgen] pub struct...) to hold onto heap-allocated memory (like Vec<T>).

2. In-Place Mutation: Instead of .to_vec() (which triggers a malloc), use methods like clear() and extend_from_slice() on existing vectors. This reuses the capacity of the vector, turning a heavy memory operation into a lightning-fast memcpy.

3. Lock Your Scales: In declarative visualization engines, the engine has to scan your entire dataset to find the Min/Max values to draw the axes. If your data is relatively bounded (like our Lorenz projection), use scale_domain() to hardcode the boundaries. This saves the engine from traversing 100,000 points purely for math scaling every frame.

4. Leverage Instancing: Let the GPU do what it's good at. By passing flat arrays of floats (Float64Array) from JS to Rust, Charton can pipe that raw geometry straight into WGPU Vertex Buffers, executing massively parallel rendering operations without CPU bottlenecking.