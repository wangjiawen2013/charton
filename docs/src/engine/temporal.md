# The Temporal Engine: High-Fidelity Data Model

In Charton, time is more than just a label—it's a high-performance coordinate system. By adopting a **"Data-as-Truth"** philosophy, Charton prioritizes the preservation of raw input signals over invasive normalization, ensuring maximum precision from ingestion to rendering.

## 1. The Physical Blueprint: Hardware-Native Storage

Charton "physicalizes" temporal data into primitive integers. This architecture ensures 100% compatibility with the **Polars/Arrow** ecosystem and enables **SIMD** (Single Instruction, Multiple Data) acceleration for coordinate calculations.

| Semantic Type | Physical Storage | Default Unit | Logic & Fidelity |
| :--- | :--- | :--- | :--- |
| **Datetime** | `i64` | Nanoseconds | Absolute UTC-based points; captures full `OffsetDateTime` precision. |
| **Date** | `i32` | Days | Calendar intervals (Epoch Days); optimized for memory (4 bytes/row). |
| **Time** | `i64` | Nanoseconds | Intra-day offset from Midnight; preserves sub-microsecond event ticks. |
| **Duration** | `i64` | Nanoseconds | Relative spans; maintains mathematical symmetry with Datetime. |

---

## 2. Core Philosophy: Data as the "Gold Standard"

### 2.1 Precision-First Ingestion
Instead of forcing data into a lossy floating-point representation or a system-defined reference point during loading, Charton treats the user's original values as immutable:

*   **Integer Domain Residency**: Data stays in the `i64/i32` domain as long as possible. This avoids the **Floating-Point Precision Trap**, where `f64` loses nanosecond-level resolution when representing large Unix timestamps (the "big number noise" problem).
*   **Zero-Copy Potential**: By matching the memory layout of modern data frames, Charton can map raw buffers directly into `ColumnVector` variants with zero re-sampling or multiplication overhead.
*   **Maximum Fidelity**: When converting from high-level objects like `time::OffsetDateTime`, Charton automatically extracts nanosecond-level integers, ensuring not a single bit of precision is discarded.

### 2.2 Late-Binding Projection
The conversion to visual coordinates (`f64`) is deferred until the **last possible moment**—the Scaling Stage:
1.  **Direct Mapping**: Scales operate directly on integer slices for range (`min/max`) detection.
2.  **On-Demand Normalization**: Conversion to `f64` happens only when calculating pixel positions. This "Late-Binding" approach ensures that even at extreme zoom levels, coordinates are derived from the highest-resolution data available.

---

## 3. The Scaling Bridge: Semantic Metadata

Charton uses `TimeUnit` not as a conversion target, but as **Metadata** that describes the inherent scale of the underlying integers.

| Unit | Scaling Factor ($val \rightarrow s$) | Practical Application |
| :--- | :--- | :--- |
| **Days** | `86400.0` | **Macro**: Geological eras, historical records, and daily business logs. |
| **Seconds** | `1.0` | **Standard**: General IoT telemetry and basic event logging. |
| **Millis** | `1e-3` | **Web**: Seamless synchronization with JavaScript `Date.now()`. |
| **Nanos** | `1e-9` | **Micro**: High-frequency trading (HFT) and sub-atomic event profiling. |

### Semantic Intelligence
Retaining the original semantic variant (e.g., `Date` vs `Datetime`) allows the engine to make intelligent UI decisions:
*   **Adaptive Tick Generation**: A `Date` column automatically aligns its axis ticks to human-friendly day/month boundaries.
*   **Unit-Aware Formatting**: The system knows a `Duration` represents a span (e.g., "+30s") while a `Time` represents a specific clock point (e.g., "14:00:30").

---

## 4. Ecosystem Synergy: Rust Data Stack

Charton is designed as the visual extension of the modern Rust data ecosystem.

*   **Polars & Arrow**: Direct ingestion of primitive buffers, respecting the `TimeUnit` and `TimeZone` metadata defined in the schema.
*   **Time Crate Integration**: Native `From` implementations for `OffsetDateTime`, `Date`, and `Time`. 
*   **Memory Efficiency**: By using `Arc<ColumnVector>` within a `Dataset`, Charton enables zero-copy data sharing across multiple threads, layers, and viewports.

---

## 5. Performance Layer

*   **SIMD Acceleration**: Continuous memory layout allows the CPU to process temporal filters, range checks, and projections in parallel batches.
*   **Validity Bitmasks**: Charton uses an independent bitmask to handle `Null` values. This eliminates the need for "Sentinel Values" (like `0` or `-1`) which could be confused with actual epoch timestamps.
*   **Thread-Safe Concurrency**: Arc-wrapped columns allow for simultaneous rendering of different views (e.g., a main chart and an overview minimap) without memory contention.

---

## 6. Summary: Fidelity Without Compromise

By moving away from intrusive pre-processing and adopting a **High-Fidelity Integer Model**, Charton achieves a critical balance: it is robust enough to hold the history of the universe in days, yet sharp enough to distinguish the individual ticks of a nanosecond-level signal—all while maintaining the absolute integrity of the user's original data.