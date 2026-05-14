# The Temporal Engine: Hybrid Precision Model

In Charton, time is more than just a label—it's a high-performance coordinate system. To balance the extreme precision required by scientific data with the practical constraints of web visualization, Charton employs a **Hybrid Precision Model** powered by **Anchored Offset Mapping (ARTM)**.

## 1. The Physical Blueprint

Charton bypasses the overhead of object-oriented time libraries by "physicalizing" data into raw integers. This ensures 100% compatibility with **Polars/Arrow** memory layouts and enables **SIMD** acceleration.

| Semantic Subtype | Physical Storage | Default Unit | Role in ARTM |
| :--- | :--- | :--- | :--- |
| **DateTime** | `i64` | Multi-Unit (ms, us, ns) | Absolute points; uses Dynamic Anchoring. |
| **Date** | `i32` | Days | Calendar intervals; saves 50% memory. |
| **Time** | `i64` | Nanoseconds | Intra-day precision; Fixed Anchor at `00:00`. |
| **Duration** | `i64` | Multi-Unit | Relative spans; Fixed Anchor at `0`. |

---

## 2. The Core Innovation: Anchored Offset Mapping (ARTM)

### 2.1 The Problem: Floating-Point Precision Trap
Standard web visualization and graphics APIs (Canvas, WebGPU, SVG) rely on `f64` (double-precision floating point) for coordinate calculations. However, `f64` uses a fixed 53-bit mantissa, meaning its precision is relative to the size of the number:

*   **At Unix Epoch ($\approx 1.7 \times 10^{18}$ ns)**: The smallest gap `f64` can represent is **~256ns**. 
*   **The Symptom**: Without ARTM, zooming into a sub-microsecond window causes "coordinate jitter" or "stepped curves" because adjacent nanosecond timestamps are rounded to the same floating-point value.

### 2.2 The Solution: Integer Anchoring
ARTM solves this by decoupling the **Reference Point** from the **Relative Motion**. It acts like a "Zero-Point Shift" performed in the lossless integer domain.

1.  **Anchor Selection ($T_0$)**: Upon ingestion, Charton selects a 64-bit integer anchor (usually the minimum value of the current domain).
2.  **Integer Subtraction**: For every data point $T_{raw}$, we compute the delta: $\Delta = T_{raw} - T_0$. 
    *   *Why?* This removes the "big number noise" (the billions of seconds since 1970) using exact CPU integer arithmetic (**zero loss**).
3.  **Visual Mapping**: The small resulting integer $\Delta$ is cast to `f64` and multiplied by the `unit_factor`.



### 2.3 Why We Use ARTM
*   **Resolution Empowerment**: By shifting the data range to start at `0`, we force the `f64` value into its highest-precision region (near the origin), where it can easily represent sub-nanosecond increments.
*   **Sub-Pixel Smoothness**: Even with `f64`, we need fractional coordinates for anti-aliasing and smooth animations. ARTM ensures these fractions are derived from precise offsets, not rounded timestamps.
*   **Immutable Fidelity**: Whether you are mapping the blink of an eye or a geological era, the relative displacement remains jitter-free.

---

## 3. The Scaling Bridge: Multi-Unit Synergy

While ARTM handles **Resolution (How clear it is)**, the `TimeUnit` handles **Range (How much it holds)**.

| Unit | Scaling Factor ($f \rightarrow s$) | Role & Benefit |
| :--- | :--- | :--- |
| **Days** | `86400.0` | **Macro-Scale**: Allows `i64` to span 580 billion years (Cosmological data). |
| **Millis** | `1e-3` | **Web Synergy**: Native alignment with JavaScript `Date` and standard logs. |
| **Nanos** | `1e-9` | **Micro-Scale**: 1ns precision for HFT, protected by ARTM up to 292 years. |

### The Benefit of Retention
By retaining the `TimeUnit` instead of force-converting everything to nanoseconds:
*   **Zero-Copy Ingestion**: We map Polars `Int64` buffers directly without re-sampling or multiplication.
*   **Adaptive Ticks**: The engine knows that for a "Date" type, the smallest meaningful tick is 1 day, preventing the UI from generating nonsensical "half-day" labels during zoom.

---

## 4. Ecosystem Synergy: Polars & Time Crate

Charton is the visual extension of the Rust data ecosystem.

*   **Polars Integration**: Directly consumes Polars `Series`, respecting the `TimeUnit` metadata to eliminate pre-processing overhead.
*   **Time Crate Support**: Provides `From<Vec<T>>` for `time::OffsetDateTime` and `time::Date`, flattening objects into primitives for **Eager Execution**.

---

## 5. Performance Layer

*   **SIMD Acceleration**: Storing data in contiguous `i32/i64` blocks allows the CPU to process thousands of offsets in a single clock cycle.
*   **Validity Bitmask**: Uses a separate bitmask for Nulls. This avoids "Sentinel Values" (like -1) and prevents "ghost points" at the 1970 Epoch.

---

## 6. Choosing the Right Type

| Data Scenario | Recommended Subtype | Why? |
| :--- | :--- | :--- |
| **Quant / HFT** | `Time` / `DateTime(ns)` | ARTM preserves 1ns precision; `Time` (0-anchor) is ultra-fast. |
| **Web Analytics** | `DateTime(ms)` | Perfect balance of performance and JS compatibility. |
| **Logistics / ERP** | `Date` | 50% memory saving; aligns with business calendar logic. |
| **System Profiling** | `Duration` | Native support for "1h 20m" formatting instead of decimal seconds. |

---

### Summary: Accuracy Without Compromise
By combining **TimeUnit-aware storage** with **Anchored Offset Mapping**, Charton achieves a rare feat: it is coarse enough to hold the history of the universe, yet sharp enough to distinguish the individual ticks of a high-frequency trade—all while maintaining the sub-pixel smoothness required for modern web interfaces.