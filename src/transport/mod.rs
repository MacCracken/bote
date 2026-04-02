//! Transport layer — codec, stdio, and feature-gated network transports.

pub(crate) mod codec;
pub use codec::{parse_request, process_message, serialize_response};

pub mod stdio;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "ws")]
pub mod ws;

#[cfg(feature = "unix")]
pub mod unix;

/// Streamable HTTP transport (MCP 2025-11-25) — single endpoint, SSE + POST,
/// session tracking, event ID resumption.
#[cfg(feature = "http")]
pub mod streamable;
