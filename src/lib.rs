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
//! - [`transport`] — Transport layer (stdio, HTTP, WebSocket, Unix socket)
//! - [`dispatch`] — Route tool calls to registered handlers

pub mod dispatch;
pub mod protocol;
pub mod registry;
pub mod stream;
pub mod transport;

mod error;
pub use error::BoteError;

pub use dispatch::{DispatchOutcome, Dispatcher};
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use registry::{ToolDef, ToolRegistry, ToolSchema};
pub use stream::{CancellationToken, ProgressSender, ProgressUpdate, StreamContext, StreamingToolHandler};

pub type Result<T> = std::result::Result<T, BoteError>;

#[cfg(test)]
mod tests;
