# The Life of a Chart

A chart in Charton is more than just a static image; it is the end product of a high-speed data transformation pipeline. From the moment you load a raw CSV to the millisecond a pixel lights up on your GPU, data undergoes a series of rigorous stages—validation, mapping, and hardware acceleration.

This section traces the "biography" of a chart, revealing how Charton's columnar architecture ensures that even millions of data points move through this lifecycle with near-zero latency.

## Data Pipeline: From Bytes to Pixels

Charton follows a strictly columnar, one-way data pipeline designed for maximum throughput and GPU efficiency. The journey consists of five distinct stages:

### 1. Ingestion (`ToDataset`)

Data enters the system through the `ToDataset` trait. Whether originating from a CSV, a Polars DataFrame, or a simple `Vec`, data is transformed into a Columnar Dataset. This stage performs type-checking and builds Validity Bitmaps to track missing values (`null`) without bloating memory.

### 2. Encoding (Visual Mapping)

In this phase, the user defines the "Grammar of Graphics." You map semantic data columns to visual channels (Encodings).

- Example: Map the `timestamp` column to the X-Axis and the `value` column to the Y-Axis.
- Charton validates these mappings against the Dataset Schema to ensure type compatibility before rendering begins.

### 3. Extraction(Zero-Copy Retrieval)

When the renderer prepares a frame, it requests data from the `Dataset` using `get_column_<T>`. Because Charton uses a columnar layout, this returns a direct slice (`&[T]`) of contiguous memory. There is no row-by-row iteration or unnecessary cloning at this stage, keeping CPU cache hits high.

### 4. Uploading (GPU Buffering)

The extracted memory slices are uploaded directly to WGPU Vertex Buffers. For types like `f64` or `f32`, this is often a raw memory copy (Bit-casting), which is the fastest possible way to move data from CPU RAM to GPU VRAM.

### 5. Drawing (Hardware Acceleration)

Finally, the GPU executes specialized Shaders. Using the uploaded buffers, the graphics hardware parallelizes the rendering process, instantly drawing hundreds of thousands (or millions) of data points as triangles, lines, or points on the screen.