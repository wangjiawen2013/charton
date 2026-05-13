# The Temporal Engine: Hybrid Precision Model

In Charton, time is more than just a label—it's a high-performance coordinate system. To balance the extreme precision required by scientific data with the practical constraints of web visualization and memory efficiency, Charton employs a **Hybrid Precision Model**.

This engine treats different temporal types with specialized physical representations, ensuring that your time-series data is both SIMD-friendly and logically intuitive.

## 1. The Physical Blueprint

Charton stores temporal data as raw integers ($i32$ or $i64$) to bypass the overhead of object-oriented time libraries during computation. This "Physicalization" is the key to our eager execution performance.

| Logical Type | Physical Type | Base Unit | Range / Capability |
| :--- | :--- | :--- | :--- |
| **Date** | `i32` | **Days** | Efficient calendar math; 100% Polars compatible. |
| **Time** | `i64` | **Nanoseconds** | Scientific-grade precision within a 24-hour window. |
| **Datetime** | `i64` | **Milliseconds** | Native alignment with JavaScript/Web ecosystems. |
| **Duration** | `i64` | **Milliseconds** | Large-span intervals (up to 290 million years). |

## 2. Design Philosophy

### Date: The Epoch Offset
Instead of using complex Julian dates or strings, Charton stores `Date` as the number of **whole days** since the Unix Epoch (January 1, 1970).
*   **Memory Efficiency**: Using $i32$ days keeps the footprint tiny (4 bytes per row).
*   **Arithmetic**: Date-shifting (e.g., "7 days later") becomes a simple integer addition.

### Time: Midnight-Relative Nanoseconds
For the `Time` variant (representing time-of-day), Charton locks the precision to **Nanoseconds** relative to `00:00:00`.
*   **Precision without Compromise**: A single day contains $86,400 \times 10^9$ nanoseconds. This fits comfortably within an $i64$.
*   **Use Case**: Ideal for high-frequency trading (HFT) or sub-millisecond sensor logs.

### Datetime & Duration: The Millisecond Bridge
`Datetime` and `Duration` are standardized to **Milliseconds**. 
*   **Web Synergy**: JavaScript's `Date` object and most browser-side libraries operate in milliseconds. Using this unit eliminates costly division cycles when sending data over WASM.
*   **The 2262 Problem**: Using nanoseconds for absolute timestamps limits the range to roughly 292 years. By using milliseconds, Charton can represent a range of $\pm 290$ million years.

## 3. High-Performance Alignment

The Temporal Engine is designed for **Eager Execution**. When a vector of objects is ingested, it is immediately "flattened" into its physical primitive.

### SIMD-Ready Layout
Because data is stored in contiguous memory blocks (e.g., `Vec<i64>`), the CPU can use **SIMD (Single Instruction, Multiple Data)** to process thousands of timestamps in a single clock cycle.

### Null Handling (The Validity Bitmask)
Charton does not use "Sentinel Values" (like -1) for null dates. Instead, it maintains a separate **Validity Bitmask**. 
*   **Physical Layer**: Nulls are often stored as `0` (Unix Epoch) in the data array to maintain memory alignment.
*   **Logic Layer**: The engine checks the bitmask; if the bit for a row is `0`, the renderer ignores that point, preventing "ghost" points at 1970-01-01.

## 4. Choosing the Right Type

| If your data is... | Recommended Type | Why? |
| :--- | :--- | :--- |
| **Stock Market (Intraday)** | `Time` | Needs nanosecond precision for ticks. |
| **Standard Time-Series** | `Datetime` | Best compatibility with web formatters. |
| **Daily Sales / Logs** | `Date` | Saves 50% memory compared to Datetime. |