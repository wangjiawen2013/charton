# Industrial Mastery

## Performance & Scaling

When a visualization library transitions from an academic prototype to an industrial-grade production tool, the primary bottleneck shifts from architectural expressiveness to raw computational performance. In enterprise environments, charts are routinely required to process millions of observation rows, compute complex statistical overlays in real time, and render under tight latency budgets. 

Charton achieves sub-millisecond execution speeds on large-scale datasets by optimizing three distinct layers: hardware-friendly data ingestion, high-performance associative hashing, and streamlined geometric compilation.

### Zero-Copy Ingestion via Polars-Aligned Memory

Naive visualization tools often store data as rows of object instances, which introduces severe memory fragmentation and continuous pointer chasing. Charton eliminates this overhead by strictly enforcing a contiguous columnar memory layout.

As defined in the core data structures, a dataset is broken down into independent vectors of strongly typed primitives (`ColumnVector` variants such as `Float64`, `Int32`, or `String`). 

* **Polars Optimization**: This layout mirrors the memory alignment used by the `Polars` DataFrame engine. When you ingest a dataset from a Polars source, Charton can perform a near-zero-cost conversion, repurposing the underlying Arrow-backed memory buffers rather than duplicating arrays.
* **Cache Locality**: By structuring data as contiguous primitive arrays (e.g., `Vec<f64>`), the CPU can efficiently pre-fetch values into its L1/L2 caches during axis scaling and coordinate transformations, maximizing hardware throughput.

### High-Performance Hashing with `ahash`

Statistical marks—such as Box Plots and Kernel Density Estimations (KDE)—require the engine to group, bin, and partition continuous data rows based on categorical channels (like `Color` or `X` slots) before any drawing occurs. In standard implementations, these associative lookups represent a critical performance sink.

To solve this, Charton replaces Rust’s standard, cryptographically secure `SipHasher` with **`ahash`** (`AHashMap` and `AHashSet`) inside its core transform pipelines (`transform_boxplot_data` and `transform_density_data`).

$$\text{Throughput Gain} = \frac{\text{AHashMap Latency}}{\text{Standard Std Hash Latency}} \approx 3\times \text{ to } 5\times \text{ Acceleration}$$

Because visualization tasks operate within a controlled local context and do not require protection against Denial-of-Service (DoS) algorithmic attacks, `ahash` leverages customized hardware instructions (like AES-NI) to generate non-cryptographic hashes at a fraction of the CPU cycles. This ensures that grouping thousands of categorical intersections completes in microseconds.

### Gap-Filling and Dense Categorical Alignment

When compiling complex multi-series statistical layouts (such as dodged box plots or stacked histograms), missing categories in certain sub-groups can break layout calculations, leading to uneven alignment grids.

Charton’s statistical transformers optimize this on-the-fly during the transformation step:
1. **Categorical Matrix Scanning**: The transformer uses an `AHashMap` to compile the global cartesian product of all available categorical keys.
2. **Deterministic Gap Insertion**: If a specific category combination lacks data points, the transformer automatically injects standard tracking boundaries or explicit `f64::NAN` rows.
3. **Downstream Predictability**: This dense padding guarantees that the layout engine receives perfectly balanced arrays, allowing visual positions to be calculated in single-pass vector operations without nested runtime checks.

### Continuous Value Bound Sampling

For computationally heavy transformations like Kernel Density Estimation (KDE), evaluating the probability density at every single raw coordinate across millions of rows is redundant and slow. Charton optimizes the transformation pipeline by isolating boundary evaluations:

* **Grid Sample Generation**: The system scans the continuous target column to establish the true minimum and maximum global boundaries.
* **Fixed Linear Interpolation**: It then projects a fixed, customizable grid (e.g., 512 discrete sampling steps) between these boundaries.
* **Vectorized KDE Execution**: The underlying kernel functions (Normal, Epanechnikov, or Uniform) are evaluated exclusively against these synthesized grid points, turning an $O(N^2)$ point-cloud computation into an $O(N \cdot \text{Grid})$ linear operational pass.

---

## Key Takeaways
* **Columnar Layouts**: Mirroring Arrow/Polars memory structures minimizes transformation copies and maximizes CPU cache locality.
* **Ahash Acceleration**: Relying on hardware-accelerated hashing makes grouping and categorizing thousands of data series practically free.
* **Fixed Grid Transformations**: Decoupling raw rows from evaluation steps prevents complex statistics (like KDE) from scaling exponentially with data size.