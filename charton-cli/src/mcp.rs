//! The MCP server: a controlled way for an agent to call Charton.
//!
//! A model gets a small set of verbs — profile the data, check a spec, render it
//! — where every answer is either a result or a diagnostic it can act on. What
//! it may touch is decided by whoever started the server.
//!
//! ```text
//! charton mcp [--data-dir DIR] [--allow-write]
//! ```
//!
//! See [`tools`] for the verbs and [`server`] for the transport.

mod protocol;
mod server;
mod tools;

pub use server::serve;
