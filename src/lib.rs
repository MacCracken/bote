//! # Bote — MCP Core Service
//!
//! Bote (German: messenger) provides the shared MCP (Model Context Protocol)
//! implementation for the AGNOS ecosystem. JSON-RPC 2.0 protocol, tool registry,
//! schema validation, and stdio transport — so individual apps don't each
//! reimplement the protocol.
//!
//! ## Modules
//!
//! - [`protocol`] — JSON-RPC 2.0 types (Request, Response, Error)
//! - [`registry`] — Tool registry with schema validation and discovery
//! - [`transport`] — Stdio transport (read JSON-RPC from stdin, write to stdout)
//! - [`dispatch`] — Route tool calls to registered handlers

pub mod dispatch;
pub mod protocol;
pub mod registry;
pub mod transport;

mod error;
pub use error::BoteError;

pub use dispatch::Dispatcher;
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use registry::{ToolDef, ToolRegistry, ToolSchema};

pub type Result<T> = std::result::Result<T, BoteError>;

#[cfg(test)]
mod tests;
