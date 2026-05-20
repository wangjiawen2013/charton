# Safety & Error Registry

In high-performance visualization systems, the most costly errors are not rendering failures, but "logical mapping errors"—such as attempting to map continuous data to a discrete aesthetic channel, or misapplying coordinate scaling logic. Charton’s architecture is built on the philosophy of "Fail-Fast at Compile-Time" and "Structured Diagnostics."

## Type-Safe Grammar and Validation

Charton’s core architecture enforces the legitimacy of its "Grammar of Graphics" at compile time, preventing developers from constructing invalid visual encodings.

- Generic Type Guards: Utilizing the state-machine pattern within `Chart<T>`, Charton locks down operational logic via Rust's type system. For instance, statistical transformations like `transform_boxplot_data` are only accessible to specific mark types. Any attempt to apply unsupported operations triggers a compile-time error.
- Channel-Scale Alignment: During the chart specification phase, the system validates the consistency between `Channel` (e.g., Color, X, Y) and the requested `Scale`. By checking that the target data field matches the scale's domain, the system eliminates runtime "undefined mapping" risks.

## Structured Error Registry
To ensure developers can rapidly diagnose complex visual issues, Charton replaces ambiguous error messages with a structured `ChartonError` enum. This registry centralizes the management of potential logical conflicts:

```rust
pub enum ChartonError {
    /// Error related to data handling or processing.
    #[error("Data error: {0}")]
    Data(String),
    /// Error related to mark definitions or configurations.
    #[error("Mark error: {0}")]
    Mark(String),
    /// Error related to encoding specifications.
    #[error("Encoding error: {0}")]
    Encoding(String),
}
```

Every error variant carries contextual metadata:

- Context-Aware Diagnostics: When the `LayoutEngine` calculates legend dimensions, if a `panel_defense_ratio` violation occurs, the registry returns a detailed report containing the current `Rect` state. This clearly identifies exactly which legend block caused the spatial overflow.
- Recoverable Diagnostics: For non-fatal data issues (e.g., outliers slightly beyond the domain), the system supports "Soft Warning" modes. These warnings are logged in the registry, allowing the chart to perform automatic clipping instead of crashing.

## Semantic Validation Pipeline
Before any `Chart` object proceeds to the render phase, it must pass through a Semantic Validation stage—a defensive "wall" designed to ensure structural integrity:

- Dimensional Validity: The system scans all `Encoding` mappings. If a `MarkPoint` is detected alongside incompatible `StackMode` parameters (usually reserved for `MarkArea` or `MarkBar`), the validator triggers a semantic error before expensive rendering begins.
- Coordinate Scale Consistency: The system verifies that the coordinate system logic (e.g., `Polar` vs. `Cartesian2D`) matches the data's scale domains, preventing logical errors like using negative radial values in a polar chart.
- CI/CD Integration: These error diagnostics can be serialized into JSON format. This allows for automated "Snapshot Testing" in CI/CD pipelines, where error-message hashes are compared to ensure that exception-handling mechanisms remain stable across library updates.

## Key Takeaways
- Compile-Time Safety: Leveraging Rust's strong type system, we intercept incorrect mapping logic during development rather than in the production runtime.
- Structured Diagnostics: The error registry carries geometric context (panel sizes, coordinate parameters), significantly reducing the time required to debug complex layouts.
- Semantic Integrity: The validation pipeline acts as a protective layer, ensuring that all generated charts adhere to the mathematical and geometric logic of the "Grammar of Graphics."