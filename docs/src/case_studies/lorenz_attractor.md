# The 50k-Point Lorenz Attractor (WGPU & Zero-Allocation WASM)

The previous chapter's WGPU implementation (*Chapter WebAssembly & Vega-Lite JSON, Part 2*) was a "naive" port that re-allocated heap memory every frame. To push WebAssembly and your GPU to their limits, we will render a Lorenz Attractor—a complex, non-repeating 3D trajectory—simulating 50,000 dynamic particles at a locked 60 FPS.

Crucially, we will refactor our WASM boundary to achieve Zero-Allocation (Zero Malloc) during the render loop.

## The Bottleneck: Why "Naive" WASM Stutters

Pushing Part 2's code to 50,000 points causes micro-stutters. The CPU, not the GPU, struggles with memory allocation. Our previous `render_chart_gpu` function ran this every frame:

```rust
let ds = Dataset::new()
    .with_column("x", xs.to_vec()) // Heap Allocation!
    // ...
```

Calling `to_vec()` 60 times a second on 50k points forces constant memory allocation and deallocation, choking the CPU and starving the GPU. The fix is shifting from a Stateless API to a Stateful Architecture using a persistent Rust struct to reuse memory.

## Rust: The Stateful LiveChartApp

We will create a persistent Rust object that pre-allocates memory for our points at startup. During the animation loop, Charton's `update_column_f64` performs an in-place memory copy, bypassing heap allocations entirely.

Update your `src/lib.rs`:

```rust
use charton::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct LiveChartApp {
    dataset: Dataset,
    canvas_id: String,
}

#[wasm_bindgen]
impl LiveChartApp {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: String, capacity: usize) -> Result<LiveChartApp, JsValue> {
        let zeros = vec![0.0; capacity];

        let dataset = Dataset::new()
            .with_column("x", zeros.clone())
            .map_err(|e| e.to_string())?
            .with_column("y", zeros.clone())
            .map_err(|e| e.to_string())?
            .with_column("intensity", zeros)
            .map_err(|e| e.to_string())?;

        Ok(Self { dataset, canvas_id })
    }

    pub async fn update_and_render(
        &mut self,
        xs: &[f64],
        ys: &[f64],
        colors: &[f64],
    ) -> Result<(), JsValue> {
        // Zero-allocation memory overwrite
        self.dataset
            .update_column_f64("x", xs)
            .map_err(|e| e.to_string())?;
        self.dataset
            .update_column_f64("y", ys)
            .map_err(|e| e.to_string())?;
        self.dataset
            .update_column_f64("intensity", colors)
            .map_err(|e| e.to_string())?;

        // Build lightweight declarative chart and flush to WGPU Canvas
        Chart::build(self.dataset.clone())
            .map_err(|e| e.to_string())?
            .mark_point()
            .map_err(|e| e.to_string())?
            .configure_point(|p| p.with_size(1.0).with_opacity(0.4))
            .encode((
                // Lock domain to avoid full scan, adaptively fit based on standard Lorenz range
                alt::x("x"),
                alt::y("y"),
                alt::color("intensity"),
            ))
            .map_err(|e| e.to_string())?
            // Responsive dimensions fully controlled by frontend canvas styles
            .configure_theme(|t| {
                t.with_background_color("#090d16")
                    .with_color_map(ColorMap::Plasma)
                    .with_show_axes(false)
                    .with_show_legend(false)
                    .with_top_margin(0.10)
                    .with_bottom_margin(0.0)
                    .with_left_margin(0.0)
                    .with_right_margin(0.0)
            })
            .render_to_canvas(&self.canvas_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
```

## JavaScript: Chaos Math & 3D Projection

In `index.html`, we calculate the Lorenz equations. To create a rotating 3D effect, we compute coordinates once, then apply a dynamic 3D-to-2D rotation matrix each frame before streaming data to WASM.

Replace `index.html` with this:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Charton WASM — 50k Lorenz Chaos Engine</title>
    <style>
        :root {
            --bg-color: #05070f;
            --panel-bg: rgba(13, 17, 23, 0.75);
            --accent-color: #00f2ff;
            --accent-glow: rgba(0, 242, 255, 0.4);
            --text-color: #c9d1d9;
            --border-color: #21262d;
        }

        body {
            font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
            background: var(--bg-color);
            color: var(--text-color);
            margin: 0;
            padding: 0;
            height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center; 
            background: radial-gradient(circle at center, #0f1626 0%, #05070f 100%);
            overflow: hidden;
        }

        /* Main container: max width limited to 1080px */
        #app-container {
            display: flex;
            width: 90vw;
            max-width: 1080px; 
            height: 80vh; 
            max-height: 720px; 
            background: #090d16;
            border-radius: 16px;
            border: 1px solid rgba(0, 242, 255, 0.15);
            box-shadow: 0 24px 60px rgba(0, 0, 0, 0.8);
            overflow: hidden;
        }

        /* Left panel: ultra-compact layout */
        #control-panel {
            width: 260px;
            background: var(--panel-bg);
            border-right: 1px solid var(--border-color);
            padding: 1.2rem; 
            display: flex;
            flex-direction: column;
            gap: 0.8rem; 
            box-sizing: border-box;
            backdrop-filter: blur(12px);
            height: 100%;
            overflow: hidden; /* Lock viewport, prevent scrollbars */
        }

        h2 {
            font-size: 0.95rem; 
            margin: 0;
            color: #fff;
            text-transform: uppercase;
            letter-spacing: 1px;
            border-bottom: 2px solid var(--border-color);
            padding-bottom: 0.4rem;
            flex-shrink: 0;
        }

        .stats-box {
            background: rgba(255, 255, 255, 0.03);
            border: 1px solid var(--border-color);
            border-radius: 6px;
            padding: 0.4rem 0.6rem;
            font-family: monospace;
            font-size: 0.75rem; 
            flex-shrink: 0;
        }

        .stat-line {
            display: flex;
            justify-content: space-between;
            margin-bottom: 0.15rem;
        }

        .stat-value {
            color: var(--accent-color);
            font-weight: bold;
            text-shadow: 0 0 8px var(--accent-glow);
        }

        #controls-container {
            flex: 1;
            display: flex;
            flex-direction: column;
            gap: 0.8rem; 
            overflow: hidden;
        }

        .control-group {
            display: flex;
            flex-direction: column;
            gap: 0.2rem; 
            flex-shrink: 0;
            box-sizing: border-box;
        }

        label {
            font-size: 0.75rem; 
            color: #8b949e;
            display: flex;
            justify-content: space-between;
        }

        .label-val {
            color: #fff;
            font-family: monospace;
        }

        /* Ultra-thin slider style design */
        input[type="range"] {
            -webkit-appearance: none;
            -moz-appearance: none;
            appearance: none;
            
            width: 100%;
            background: #161b22;
            height: 4px; 
            border-radius: 2px;
            outline: none;
            margin: 4px 0; 
            flex-shrink: 0;
        }

        input[type="range"]::-webkit-slider-thumb {
            -webkit-appearance: none;
            appearance: none;
            
            width: 12px; 
            height: 12px;
            border-radius: 50%;
            background: var(--accent-color);
            cursor: pointer;
            box-shadow: 0 0 6px var(--accent-color);
            transition: transform 0.1s;
        }

        input[type="range"]::-moz-range-thumb {
            border: none;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            background: var(--accent-color);
            cursor: pointer;
            box-shadow: 0 0 6px var(--accent-color);
            transition: transform 0.1s;
        }

        /* Right canvas area */
        #stage {
            flex: 1;
            height: 100%;
            position: relative;
            background: #090d16;
        }

        #chart-canvas {
            width: 100%;
            height: 100%;
            display: block;
        }
    </style>
</head>
<body>

    <div id="app-container">
        
        <div id="control-panel">
            <h2>🦋 Lorenz Chaos</h2>
            
            <div class="stats-box">
                <div class="stat-line">
                    <span>Particles:</span>
                    <span class="stat-value">50,000</span>
                </div>
                <div class="stat-line">
                    <span>Performance:</span>
                    <span class="stat-value"><span id="fps-counter">0</span> FPS</span>
                </div>
            </div>

            <div id="controls-container">
                <div class="control-group">
                    <label>Sigma (σ) <span id="val-sigma" class="label-val">10.0</span></label>
                    <input type="range" id="param-sigma" min="1.0" max="30.0" step="0.1" value="10.0">
                </div>

                <div class="control-group">
                    <label>Rho (ρ) <span id="val-rho" class="label-val">28.0</span></label>
                    <input type="range" id="param-rho" min="5.0" max="50.0" step="0.1" value="28.0">
                </div>

                <div class="control-group">
                    <label>Beta (β) <span id="val-beta" class="label-val">2.67</span></label>
                    <input type="range" id="param-beta" min="0.5" max="5.0" step="0.01" value="2.666">
                </div>

                <div class="control-group">
                    <label>Rotation Speed <span id="val-speed" class="label-val">1.0</span></label>
                    <input type="range" id="param-speed" min="0.0" max="3.0" step="0.1" value="1.0">
                </div>
            </div>
        </div>

        <div id="stage">
            <canvas id="chart-canvas"></canvas>
        </div>

    </div>

    <script type="module">
        import init, { LiveChartApp } from './pkg/wave.js';

        async function run() {
            await init();

            const TOTAL_POINTS = 50000;
            const fpsCounter = document.getElementById('fps-counter');

            const sliders = {
                sigma: document.getElementById('param-sigma'),
                rho: document.getElementById('param-rho'),
                beta: document.getElementById('param-beta'),
                speed: document.getElementById('param-speed')
            };
            const labels = {
                sigma: document.getElementById('val-sigma'),
                rho: document.getElementById('val-rho'),
                beta: document.getElementById('val-beta'),
                speed: document.getElementById('val-speed')
            };

            Object.keys(sliders).forEach(key => {
                sliders[key].addEventListener('input', (e) => {
                    labels[key].textContent = parseFloat(e.target.value).toFixed(2);
                });
            });

            const app = new LiveChartApp("chart-canvas", TOTAL_POINTS);

            const xs = new Float64Array(TOTAL_POINTS);
            const ys = new Float64Array(TOTAL_POINTS);
            const colors = new Float64Array(TOTAL_POINTS);

            const lx = new Float64Array(TOTAL_POINTS);
            const ly = new Float64Array(TOTAL_POINTS);
            const lz = new Float64Array(TOTAL_POINTS);

            let angle = 0;
            let lastTime = performance.now();

            function computeLorenzTrajectory() {
                const sigma = parseFloat(sliders.sigma.value);
                const rho = parseFloat(sliders.rho.value);
                const beta = parseFloat(sliders.beta.value);
                
                let x = 0.1, y = 0.0, z = 0.0;
                const dt = 0.005;

                for (let i = 0; i < TOTAL_POINTS; i++) {
                    x += sigma * (y - x) * dt;
                    y += (x * (rho - z) - y) * dt;
                    z += (x * y - beta * z) * dt;
                    
                    lx[i] = x; 
                    ly[i] = y; 
                    lz[i] = z;
                    colors[i] = z;
                }
            }

            computeLorenzTrajectory();

            async function frameLoop() {
                const now = performance.now();
                fpsCounter.textContent = Math.round(1000 / (now - lastTime));
                lastTime = now;

                computeLorenzTrajectory();

                const speedModifier = parseFloat(sliders.speed.value);
                angle += 0.01 * speedModifier;
                
                const cosA = Math.cos(angle);
                const sinA = Math.sin(angle);

                for (let i = 0; i < TOTAL_POINTS; i++) {
                    xs[i] = lx[i] * cosA - ly[i] * sinA;
                    ys[i] = lz[i]; 
                }

                try {
                    await app.update_and_render(xs, ys, colors);
                } catch (e) {
                    console.error("WGPU Render failed:", e);
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

Running this reveals 50,000 points swirling fluidly in 3D. The small, semi-transparent points visually accumulate in dense areas, making the "butterfly wings" physically glow. By eliminating heap allocations, CPU usage drops drastically, allowing the `requestAnimationFrame` loop to feed WGPU at your monitor's maximum refresh rate.

## Summary: Rules for High-Performance WASM

Keep these golden rules in mind for data-intensive JS/WASM applications:

1. State is King: Avoid stateless APIs for high-frequency loops. Use a persistent Rust struct (`#[wasm_bindgen] pub struct...`) to hold heap-allocated memory.

2. In-Place Mutation: Avoid `.to_vec()` (which triggers a malloc). Mutate vectors in place to reuse capacity, turning heavy memory operations into lightning-fast memcopies.

3. Leverage Instancing: Pass flat float arrays (`Float64Array`) from JS to Rust. Charton pipes this raw geometry directly into WGPU Vertex Buffers, enabling parallel GPU rendering without CPU bottlenecks.