
# Installation

Charton is built with a modular, pay-for-what-you-use architecture. By leveraging Cargo features, you can finely tune your compilation to match your production constraints—whether you are deploying to a resource-constrained WebAssembly environment, building high-throughput multi-threaded backends, or bridging to existing Python data science pipelines.

## Adding Charton to Your Project

To get started with the standard, single-threaded configuration, add Charton to your `Cargo.toml`:

```toml
[dependencies]
charton = "0.5"
```

## Tailoring with Cargo Features

For production deployments, we strongly recommend enabling specific features to unlock advanced performance and backend rendering capabilities:

```toml
[dependencies]
# Example 1: Enable multi-threaded data processing for massive Polars DataFrames
charton = { version = "0.5", features = ["parallel"] }

# Example 2: Enable native image and document encoders for direct file exports
charton = { version = "0.5", features = ["png", "pdf"] }

# Example 3: Enable the high-speed interop bridge for Altair/Matplotlib integration
charton = { version = "0.5", features = ["bridge"] }
```

## Feature Flag Maxtrix

| Feature Flag | Core Mechanism & Dependencies | Target Use Case |
| :--- | :--- | :--- |
| `parallel` | Activates Rayon-backed parallel computation for scale arbitration and geometry derivation. | Processing high-density data, such as financial market depth charts or massive scatter plots. |
| `png` | Pulls in native PNG encoding backends, activating the `.save("out.png")` API. | Automated server-side report generation and automated dashboard asset caching. |
| `pdf` | Integrates a vector PDF document renderer, activating the `.save("out.pdf")` API. | Generating publication-quality vector figures compliant with top-tier scientific journals (e.g., NEJM). |
| `bridge` | Initiates a high-speed IPC channel to map Charton layers directly onto Python Altair/Matplotlib abstract syntax trees. | Dual-stack data pipelines, gradual migration, or reusing mature, domain-specific Python plotting scripts. |