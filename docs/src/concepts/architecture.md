# System Architecture

Charton’s architecture is designed around the principle of Separation of Concerns. To transform a high-level user specification into a physical image, the system utilizes a four-layer pipeline: Input, Core, Render, and Output.

## The Four-Layer Architecture

The following diagram illustrates the flow of information through the system:

### I. The Input Layer (Data Ingestion)

The Input layer acts as the entry point for all data. Charton is built on the Arrow memory format, allowing for zero-copy integration with high-performance data libraries.
* Data Sources: Accepts structured dataframes or serialized streams.
* Bridge System: Provides a language-agnostic interface that allows the core engine to receive data from different environments (e.g., Python) without version conflicts.

### II. The Core Layer (The Specification Engine)

This is the "Brain" of Charton. It is responsible for the logical interpretation of the user’s intent. It consists of three primary sub-systems:
* Specification (Spec): Stores the "Blueprints" of the chart—which columns go to which axes, which colors are used, and which geometric marks are applied.
* Scale Arbitration: A critical phase where the engine scans all layers to find global data boundaries (min/max or unique categories) to ensure all layers are visually synchronized.
* Aesthetic Mapping: Resolves abstract data values into normalized [0, 1] ratios, which are later mapped to physical properties like hex codes or point shapes.

### III. The Render Layer (The Geometric Factory)

Once the Core layer has resolved the mathematical logic, the Render layer converts these abstractions into geometry.

* Coordinate Transformation: Translates normalized data into physical canvas coordinates based on the selected system (e.g., Cartesian, Polar, or Geographic).
* Layout Engine: A "Flex-box" style system that greedily calculates space for marginalia—axes, titles, and legends—ensuring the plot panel occupies the remaining space correctly.
* Mark Generation: Generates specific instructions for paths, circles, and rectangles.

### IV. The Output Layer (The Backend)
The final layer translates geometric instructions into a specific file format or display buffer.

* Vector Output: Generates SVG or PDF files for infinite scalability and web integration.
* Raster Output: Renders high-performance PNG or JPEG images for reports and dashboards.
* Specification Output: Can export the entire chart state as a JSON specification (compatible with Vega-Lite) for use in frontend applications.

## The Visualization Lifecycle

Understanding the architecture requires looking at the Lifecycle of a single chart:
1. Definition Phase: The user defines the `Mark` and `Encoding`.
2. Training Phase: The system "trains" its scales by looking at the data limits of every layer.
3. Resolution Phase: Scales are "frozen," and layout constraints (like the width of the Y-axis labels) are calculated.
4. Assembly Phase: The layout engine allocates space, and the coordinate system maps the data to the final "Rect" of the plot.
5. Drawing Phase: The backend executes the final draw calls.

**Key Takeaway**

By decoupling the Core Logic (what the data means) from the Render Logic (where the pixels go), Charton allows users to swap coordinate systems or output formats without ever changing their data analysis code.