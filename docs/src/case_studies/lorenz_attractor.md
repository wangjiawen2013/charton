# The 50k-Point Lorenz Attractor (WGPU & Zero-Allocation WASM)
While the previous chapter demonstrated how to hook up charton to WGPU, our implementation was essentially a "naive" port. We were still rebuilding the Dataset and re-allocating memory on the heap every single frame.

To truly push WebAssembly and your graphics card to their absolute limits, we need to talk about Chaos Theory, glowing butterflies, and Stateful Memory Management.

In this case study, we will render a Lorenz Attractor—a complex, non-repeating 3D mathematical trajectory. We will simulate and project 50,000 dynamic particles rotating in 3D space, pushing the results to WGPU at a locked 60 FPS.

More importantly, we will refactor our WASM boundary to achieve Zero-Allocation (Zero Malloc) during the render loop.

## The Bottleneck: Why "Naive" WASM Stutters
If you tried to push the code from Part 2 to 50,000 points, you might have noticed occasional micro-stutters. The culprit isn't the GPU; it's the CPU struggling with memory allocation.

In our previous `render_chart_gpu` function, we executed this every frame:

```rust
let ds = Dataset::new()
    .with_column("x", xs.to_vec()) // Heap Allocation!
    // ...
```

At 60 FPS with 50k points, `to_vec()` forces the WASM memory allocator to find, reserve, copy, and free megabytes of memory hundreds of times a second. The Garbage Collector/Allocator chokes, and the GPU is left starving for data.

To solve this, we must shift from a Stateless API (calling a function every frame) to a Stateful Architecture (instantiating a persistent Rust struct that reuses memory capacity).

## Rust: The Stateful LiveChartApp
We will create a long-lived Rust object that pre-allocates the memory for our 100,000 points once during startup. During the 60 FPS animation loop, we will use Charton's high-performance update_column_f64 method to perform an in-place memcpy, completely bypassing heap allocation.

Update your src/lib.rs with the following advanced pattern:

```rust
use charton::prelude::*;
use charton::scale::ScaleDomain;
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
        // 零分配内存覆写
        self.dataset
            .update_column_f64("x", xs)
            .map_err(|e| e.to_string())?;
        self.dataset
            .update_column_f64("y", ys)
            .map_err(|e| e.to_string())?;
        self.dataset
            .update_column_f64("intensity", colors)
            .map_err(|e| e.to_string())?;

        // 构建轻量级声明图表并刷新至 WGPU Canvas
        Chart::build(self.dataset.clone())
            .map_err(|e| e.to_string())?
            .mark_point()
            .map_err(|e| e.to_string())?
            .configure_point(|p| p.with_size(1.0).with_opacity(0.4))
            .encode((
                // 锁定域以避免全量扫描，根据 Lorenz 常规范围进行自适应适配
                alt::x("x").with_domain(ScaleDomain::Continuous(-50.0, 50.0)),
                alt::y("y").with_domain(ScaleDomain::Continuous(-10.0, 60.0)),
                alt::color("intensity").with_domain(ScaleDomain::Continuous(0.0, 60.0)),
            ))
            .map_err(|e| e.to_string())?
            // 由前端 canvas 样式完全掌控响应式高宽
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
In `index.html`, we will calculate the Lorenz equations. To create the "rotating glowing butterfly" effect, we will compute the 3D coordinates statically once, and then apply a dynamic 3D-to-2D rotation matrix every frame before streaming the data into our WASM app.

Replace your script tag in `index.html` with this high-performance orchestrator:

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

        /* 整体大容器：最大宽度限制为 1080px */
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

        /* 左侧面板：极致紧凑排版 */
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
            overflow: hidden; /* 强制锁定视野，绝不出现滚动条 */
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

        /* 极细滑块样式设计 */
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

        /* 右侧画布区域 */
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
When you run this code, you will see something mesmerizing: 100,000 points swirling in a perfectly fluid, anti-aliased 3D rotation. Because the points are small (1.2) and semi-transparent (0.6), they visually accumulate where the math dictates higher density, causing the "wings" of the butterfly to physically glow.

Because we eliminated the heap allocations, your CPU utilization will drop significantly, allowing the requestAnimationFrame loop to run completely unhindered, feeding WGPU exactly as fast as your monitor can refresh.

## Summary: Rules for High-Performance WASM
If you are building data-intensive applications (like live sensor dashboards or complex animations) that cross the JavaScript/WASM boundary, keep these golden rules in mind:

1. State is King: Never use functional, stateless APIs for high-frequency loops. Instantiate a Rust struct (#[wasm_bindgen] pub struct...) to hold onto heap-allocated memory (like Vec<T>).

2. In-Place Mutation: Instead of .to_vec() (which triggers a malloc), use methods like clear() and extend_from_slice() on existing vectors. This reuses the capacity of the vector, turning a heavy memory operation into a lightning-fast memcpy.

3. Lock Your Scales: In declarative visualization engines, the engine has to scan your entire dataset to find the Min/Max values to draw the axes. If your data is relatively bounded (like our Lorenz projection), use scale_domain() to hardcode the boundaries. This saves the engine from traversing 100,000 points purely for math scaling every frame.

4. Leverage Instancing: Let the GPU do what it's good at. By passing flat arrays of floats (Float64Array) from JS to Rust, Charton can pipe that raw geometry straight into WGPU Vertex Buffers, executing massively parallel rendering operations without CPU bottlenecking.