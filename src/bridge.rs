//! This module provides the core bridging logic between Rust and Python.
//!
//! Structure:
//! - `mod.rs`: Module entry point: defines the `PyRendererExecutor` trait and shared types.
//! - `py_session.rs`: Python session management: handles GIL, dependency checks, and scope injection.
//! - `converter.rs`: Data bridge conversion: core logic for converting `Dataset` to Python objects.
//! - `altair.rs`: Altair implementation: leverages PyO3 to generate JSON/SVG.
//! - `matplotlib.rs`: Matplotlib implementation: leverages PyO3 to generate plots.

pub mod altair;
pub mod converter;
pub mod matplotlib;
pub mod py_session;

// ... Your trait definitions and shared logic here ...
