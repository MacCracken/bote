//! Streamable HTTP transport (MCP 2025-11-25).
//!
//! Single endpoint serving both POST (request/response) and GET (SSE stream).
//! Supports stream resumption via `Last-Event-ID` header and session tracking
//! via `MCP-Session-Id`.
//!
//! ## Spec requirements
//!
//! - POST to endpoint: JSON-RPC request → JSON-RPC response (or SSE stream)
//! - GET to endpoint: opens SSE stream for server-initiated messages
//! - `MCP-Protocol-Version` header required on all requests
//! - `MCP-Session-Id` header returned on initialize, required on subsequent requests
//! - SSE events carry `id` field for resumption
//! - `Last-Event-ID` header on GET resumes from that point
//! - Server primes with empty SSE event
//! - Server sends `retry:` before closing

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Configuration for the streamable HTTP transport.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StreamableConfig {
    /// Listen address.
    pub addr: SocketAddr,
    /// MCP endpoint path (e.g. "/mcp").
    pub path: String,
    /// Allowed Origin values for DNS rebinding protection.
    pub allowed_origins: Vec<String>,
    /// SSE retry hint in milliseconds (sent before closing stream).
    pub retry_ms: u64,
}

impl StreamableConfig {
    #[must_use]
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            path: "/mcp".into(),
            allowed_origins: vec!["*".into()], // permissive default for dev
            retry_ms: 5000,
        }
    }

    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    #[must_use]
    pub fn with_allowed_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    #[must_use]
    pub fn with_retry_ms(mut self, ms: u64) -> Self {
        self.retry_ms = ms;
        self
    }
}

/// Monotonically increasing event ID generator for SSE resumption.
#[derive(Debug, Default)]
pub struct EventIdGenerator {
    counter: AtomicU64,
}

impl EventIdGenerator {
    /// Generate the next event ID.
    #[must_use]
    pub fn next(&self) -> String {
        let id = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("evt-{id}")
    }
}

/// An SSE event with resumption support.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StreamEvent {
    /// Unique event ID for resumption.
    pub id: String,
    /// Event type (e.g. "message").
    pub event: String,
    /// JSON data payload.
    pub data: String,
}

impl StreamEvent {
    /// Create a new stream event.
    #[must_use]
    pub fn new(id: String, data: impl Into<String>) -> Self {
        Self {
            id,
            event: "message".into(),
            data: data.into(),
        }
    }

    /// Create the priming event (empty data, sent on connection open).
    #[must_use]
    pub fn primer(id: String) -> Self {
        Self {
            id,
            event: "message".into(),
            data: String::new(),
        }
    }
}

/// Resumption buffer — stores recent events for clients reconnecting
/// with `Last-Event-ID`.
pub struct ResumptionBuffer {
    /// Events keyed by ID, in insertion order.
    events: std::sync::RwLock<Vec<StreamEvent>>,
    /// Maximum events to buffer.
    max_size: usize,
}

impl ResumptionBuffer {
    /// Create a buffer with the given capacity.
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        Self {
            events: std::sync::RwLock::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    /// Push an event into the buffer. Evicts oldest if full.
    pub fn push(&self, event: StreamEvent) {
        let mut events = self.events.write().expect("buffer lock poisoned");
        if events.len() >= self.max_size {
            events.remove(0);
        }
        events.push(event);
    }

    /// Get all events after the given ID (for resumption).
    /// Returns empty vec if the ID is not found (too old, evicted).
    #[must_use]
    pub fn events_after(&self, last_event_id: &str) -> Vec<StreamEvent> {
        let events = self.events.read().expect("buffer lock poisoned");
        let pos = events.iter().position(|e| e.id == last_event_id);
        match pos {
            Some(idx) => events[idx + 1..].to_vec(),
            None => {
                warn!(
                    last_event_id = %last_event_id,
                    "Last-Event-ID not found in buffer — client may have missed events"
                );
                Vec::new()
            }
        }
    }

    /// Number of buffered events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.read().expect("buffer lock poisoned").len()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ResumptionBuffer {
    fn default() -> Self {
        Self::new(1000)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_id_generator_monotonic() {
        let id_gen = EventIdGenerator::default();
        let a = id_gen.next();
        let b = id_gen.next();
        let c = id_gen.next();
        assert_eq!(a, "evt-0");
        assert_eq!(b, "evt-1");
        assert_eq!(c, "evt-2");
    }

    #[test]
    fn stream_event_new() {
        let e = StreamEvent::new("evt-0".into(), r#"{"result":"ok"}"#);
        assert_eq!(e.id, "evt-0");
        assert_eq!(e.event, "message");
        assert!(e.data.contains("ok"));
    }

    #[test]
    fn stream_event_primer() {
        let e = StreamEvent::primer("evt-0".into());
        assert!(e.data.is_empty());
    }

    #[test]
    fn resumption_buffer_push_and_retrieve() {
        let buf = ResumptionBuffer::new(10);
        buf.push(StreamEvent::new("evt-0".into(), "a"));
        buf.push(StreamEvent::new("evt-1".into(), "b"));
        buf.push(StreamEvent::new("evt-2".into(), "c"));

        let after = buf.events_after("evt-0");
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].id, "evt-1");
        assert_eq!(after[1].id, "evt-2");
    }

    #[test]
    fn resumption_buffer_after_last_returns_empty() {
        let buf = ResumptionBuffer::new(10);
        buf.push(StreamEvent::new("evt-0".into(), "a"));
        let after = buf.events_after("evt-0");
        assert!(after.is_empty());
    }

    #[test]
    fn resumption_buffer_unknown_id_returns_empty() {
        let buf = ResumptionBuffer::new(10);
        buf.push(StreamEvent::new("evt-0".into(), "a"));
        let after = buf.events_after("evt-999");
        assert!(after.is_empty());
    }

    #[test]
    fn resumption_buffer_eviction() {
        let buf = ResumptionBuffer::new(3);
        buf.push(StreamEvent::new("evt-0".into(), "a"));
        buf.push(StreamEvent::new("evt-1".into(), "b"));
        buf.push(StreamEvent::new("evt-2".into(), "c"));
        buf.push(StreamEvent::new("evt-3".into(), "d")); // evicts evt-0

        assert_eq!(buf.len(), 3);
        let after = buf.events_after("evt-0"); // evt-0 is gone
        assert!(after.is_empty());

        let after = buf.events_after("evt-1");
        assert_eq!(after.len(), 2);
    }

    #[test]
    fn config_builder() {
        let cfg = StreamableConfig::new("127.0.0.1:8090".parse().unwrap())
            .with_path("/v1/mcp")
            .with_allowed_origins(vec!["http://localhost:3000".into()])
            .with_retry_ms(10000);
        assert_eq!(cfg.path, "/v1/mcp");
        assert_eq!(cfg.allowed_origins, vec!["http://localhost:3000"]);
        assert_eq!(cfg.retry_ms, 10000);
    }
}
