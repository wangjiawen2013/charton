### The Nanosecond Contract (`i64`)
Charton adopts a strict Single-Truth Input Policy for temporal data. To ensure maximum performance and eliminate ambiguity, the `TemporalScale` exclusively accepts 64-bit signed integers (`i64`) representing Unix Nanoseconds.

* Epoch: 1970-01-01 00:00:00 UTC.
* Unit: $1 \text{ nanosecond} = 10^{-9} \text{ seconds}$.

By standardizing on the finest common granularity in modern computing, Charton avoids the overhead of unit conversion and provides a predictable interface for high-frequency data.

### Interoperability with Polars/Arrow
This design is intentionally aligned with the Apache Arrow memory model and Polars datetime`[ns]` series.

* Zero-Copy Potential: Since Polars stores datetime data as `i64` nanoseconds internally, Charton can ingest large datasets from DataFrames with near-zero transformation cost.
* Ecosystem Harmony: Users working with Rust's data science stack can pass raw underlying buffers directly into Charton, bypassing expensive string parsing or object construction.

### Temporal Boundaries (The 292-Year Limit)
Using `i64` for nanoseconds introduces a physical boundary for "Calendar-aware" time:

* Lower Bound: ~1677-09-21 (Unix -9,223,372,036,854,775,808 ns)
* Upper Bound: ~2262-04-11 (Unix 9,223,372,036,854,775,807 ns)

For 99% of modern applications—including financial history, IoT logs, and human lifespans—this range is more than sufficient. Data falling outside this range is treated as Deep Time, which triggers a semantic fallback to numerical scaling (see Section 2.1.2).