//! # Bote — MCP Core Service
//!
//! Bote (German: messenger) provides the shared MCP (Model Context Protocol)
//! implementation for the AGNOS ecosystem. JSON-RPC 2.0 protocol, tool registry,
//! schema validation, and dispatch — so individual apps don't each reimplement
//! the protocol.
//!
//! ## Feature Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `http` | HTTP transport via axum (POST + SSE streaming) |
//! | `ws` | WebSocket transport via tokio-tungstenite |
//! | `unix` | Unix domain socket transport |
//! | `all-transports` | Enables `http`, `ws`, and `unix` |
//! | `audit` | Audit logging via libro hash-linked chain |
//! | `events` | Event publishing via majra pub/sub |
//! | `bridge` | TypeScript bridge with CORS and MCP result formatting |
//! | `discovery` | Cross-node tool discovery via majra pub/sub |
//! | `sandbox` | Tool sandboxing via kavach isolation backends |
//! | `full` | All transports + audit + events + bridge + discovery + sandbox |
//!
//! None are enabled by default — enable only what you need.
//!
//! ## Modules
//!
//! - [`protocol`] — JSON-RPC 2.0 types (Request, Response, Error)
//! - [`registry`] — Tool registry with schema validation, versioning, and discovery
//! - [`schema`] — JSON Schema compilation and typed validation
//! - [`dispatch`] — Route tool calls to registered handlers (with dynamic registration)
//! - [`stream`] — Streaming primitives (progress, cancellation)
//! - [`transport`] — Transport layer (stdio, HTTP, WebSocket, Unix socket)
//! - [`audit`] — Audit logging trait and libro integration
//! - [`events`] — Event publishing trait and majra integration
//! - [`bridge`] — TypeScript bridge with CORS and MCP result formatting
//! - [`discovery`] — Cross-node tool discovery via majra pub/sub
//! - [`sandbox`] — Tool sandboxing via kavach isolation backends

pub mod audit;
pub mod dispatch;
pub mod events;
pub mod protocol;
pub mod registry;
pub mod schema;
pub mod stream;
pub mod transport;

#[cfg(feature = "bridge")]
pub mod bridge;

#[cfg(feature = "host")]
pub mod host;

#[cfg(feature = "discovery")]
pub mod discovery;

#[cfg(feature = "sandbox")]
pub mod sandbox;

mod error;
pub use error::BoteError;

pub use audit::{AuditSink, ToolCallEvent};
pub use dispatch::{DispatchOutcome, Dispatcher};
pub use events::EventSink;
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use registry::{ToolDef, ToolRegistry, ToolSchema};
pub use schema::{CompiledSchema, PropertyDef, SchemaType};
pub use stream::{
    CancellationToken, ProgressSender, ProgressUpdate, StreamContext, StreamingToolHandler,
};

pub type Result<T> = std::result::Result<T, BoteError>;

#[cfg(test)]
mod tests;

/// Compile-time assertions that public types are Send + Sync.
#[cfg(test)]
mod send_sync_assertions {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn public_types_are_send_sync() {
        assert_send::<super::BoteError>();
        assert_sync::<super::BoteError>();
        assert_send::<super::Dispatcher>();
        assert_sync::<super::Dispatcher>();
        assert_send::<super::ToolRegistry>();
        assert_sync::<super::ToolRegistry>();
        assert_send::<super::ToolDef>();
        assert_sync::<super::ToolDef>();
        assert_send::<super::ToolSchema>();
        assert_sync::<super::ToolSchema>();
        assert_send::<super::JsonRpcRequest>();
        assert_sync::<super::JsonRpcRequest>();
        assert_send::<super::JsonRpcResponse>();
        assert_sync::<super::JsonRpcResponse>();
        assert_send::<super::JsonRpcError>();
        assert_sync::<super::JsonRpcError>();
        assert_send::<super::CancellationToken>();
        assert_sync::<super::CancellationToken>();
        assert_send::<super::ProgressUpdate>();
        assert_sync::<super::ProgressUpdate>();
        assert_send::<super::ProgressSender>();
        assert_sync::<super::ProgressSender>();
        assert_send::<super::StreamContext>();
        assert_sync::<super::StreamContext>();
        assert_send::<super::ToolCallEvent>();
        assert_sync::<super::ToolCallEvent>();
    }
}
