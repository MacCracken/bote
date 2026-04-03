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

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::BoteError;
use crate::dispatch::{DispatchOutcome, Dispatcher};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::session::{MCP_SESSION_ID_HEADER, SessionStore};
use crate::stream::CancellationToken;
use crate::transport::codec;
use crate::transport::middleware;

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
    /// Session timeout. `None` disables session enforcement.
    pub session_timeout: Option<Duration>,
}

impl StreamableConfig {
    #[must_use]
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            path: "/mcp".into(),
            allowed_origins: vec!["*".into()], // permissive default for dev
            retry_ms: 5000,
            session_timeout: Some(Duration::from_secs(3600)),
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

    #[must_use]
    pub fn with_session_timeout(mut self, timeout: Duration) -> Self {
        self.session_timeout = Some(timeout);
        self
    }

    #[must_use]
    pub fn without_sessions(mut self) -> Self {
        self.session_timeout = None;
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
        let mut events = self.events.write().unwrap_or_else(|e| e.into_inner());
        if events.len() >= self.max_size {
            events.remove(0);
        }
        events.push(event);
    }

    /// Get all events after the given ID (for resumption).
    /// Returns empty vec if the ID is not found (too old, evicted).
    #[must_use]
    pub fn events_after(&self, last_event_id: &str) -> Vec<StreamEvent> {
        let events = self.events.read().unwrap_or_else(|e| e.into_inner());
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
        self.events.read().unwrap_or_else(|e| e.into_inner()).len()
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
// Axum router + handlers
// ---------------------------------------------------------------------------

/// Default prune interval for session cleanup.
const SESSION_PRUNE_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Clone)]
struct StreamableState {
    dispatcher: Arc<Dispatcher>,
    active: Arc<std::sync::Mutex<HashMap<String, CancellationToken>>>,
    session_store: Option<Arc<SessionStore>>,
    allowed_origins: Vec<String>,
    event_ids: Arc<EventIdGenerator>,
    resumption: Arc<ResumptionBuffer>,
    retry_ms: u64,
    #[cfg(feature = "auth")]
    token_validator: Option<Arc<dyn crate::auth::TokenValidator>>,
    #[cfg(feature = "auth")]
    resource_metadata_url: Option<String>,
}

/// Start a streamable HTTP server on a single MCP endpoint.
///
/// - `POST {path}` — JSON-RPC request → response (or SSE stream)
/// - `GET {path}` — open SSE stream for server-initiated messages
///
/// Runs until the `shutdown` future resolves.
pub async fn serve(
    dispatcher: Arc<Dispatcher>,
    config: StreamableConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> crate::Result<()> {
    let session_store = config
        .session_timeout
        .map(|t| Arc::new(SessionStore::new(t)));

    let app = streamable_router(dispatcher, &config, session_store.clone());

    // Spawn session prune task.
    let prune_handle = session_store.map(|store| {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(SESSION_PRUNE_INTERVAL);
            loop {
                interval.tick().await;
                let pruned = store.prune_expired();
                if pruned > 0 {
                    tracing::info!(pruned, "pruned expired sessions");
                }
            }
        })
    });

    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .map_err(|e| BoteError::BindFailed(e.to_string()))?;

    tracing::info!(addr = %config.addr, path = %config.path, "streamable transport listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(BoteError::Io)?;

    if let Some(handle) = prune_handle {
        handle.abort();
    }

    tracing::info!("streamable transport shut down");
    Ok(())
}

/// Build the streamable transport axum router. Exposed for testing.
#[must_use = "build the axum router for the streamable transport"]
pub fn streamable_router(
    dispatcher: Arc<Dispatcher>,
    config: &StreamableConfig,
    session_store: Option<Arc<SessionStore>>,
) -> Router {
    let state = StreamableState {
        dispatcher,
        active: Arc::new(std::sync::Mutex::new(HashMap::new())),
        session_store,
        allowed_origins: config.allowed_origins.clone(),
        event_ids: Arc::new(EventIdGenerator::default()),
        resumption: Arc::new(ResumptionBuffer::default()),
        retry_ms: config.retry_ms,
        #[cfg(feature = "auth")]
        token_validator: None,
        #[cfg(feature = "auth")]
        resource_metadata_url: None,
    };
    Router::new()
        .route(&config.path, routing::post(handle_post).get(handle_get))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// POST handler — JSON-RPC request → response (or SSE stream)
// ---------------------------------------------------------------------------

async fn handle_post(
    State(state): State<StreamableState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    // --- Middleware ---
    if let Err(resp) = middleware::check_origin(&headers, &state.allowed_origins) {
        return resp;
    }
    if let Err(resp) = middleware::check_protocol_version_required(&headers) {
        return resp;
    }

    let is_initialize = serde_json::from_str::<JsonRpcRequest>(&body)
        .map(|r| r.method == "initialize")
        .unwrap_or(false);

    if let Err(resp) = middleware::check_session(&headers, &state.session_store, is_initialize) {
        return resp;
    }

    #[cfg(feature = "auth")]
    if let Err(resp) = middleware::check_bearer(
        &headers,
        &state.token_validator,
        &state.resource_metadata_url,
    ) {
        return resp;
    }

    // --- Request handling ---
    if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&body) {
        // Cancellation.
        if req.method == "$/cancelRequest" {
            if let Some(target_id) = req.params.get("id").and_then(|v| v.as_str())
                && let Some(token) = state
                    .active
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(target_id)
            {
                token.cancel();
            }
            return StatusCode::NO_CONTENT.into_response();
        }

        // Streaming tool → SSE.
        if req.method == "tools/call"
            && let Some(tool_name) = req.params.get("name").and_then(|v| v.as_str())
            && state.dispatcher.is_streaming_tool(tool_name)
        {
            return handle_post_streaming(state, req).into_response();
        }

        // Initialize — create session.
        if req.method == "initialize"
            && let Some(store) = &state.session_store
        {
            let protocol_version = req
                .params
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("2025-11-25")
                .to_string();
            let session_id = store.create(protocol_version);
            let dispatcher = Arc::clone(&state.dispatcher);
            let result =
                tokio::task::spawn_blocking(move || codec::process_message(&body, &dispatcher))
                    .await
                    .expect("dispatch task panicked");
            return match result {
                Some(json) => (
                    StatusCode::OK,
                    [
                        ("content-type", "application/json"),
                        (MCP_SESSION_ID_HEADER, &session_id),
                    ],
                    json,
                )
                    .into_response(),
                None => StatusCode::NO_CONTENT.into_response(),
            };
        }
    }

    // Non-streaming dispatch.
    let dispatcher = Arc::clone(&state.dispatcher);
    let result = tokio::task::spawn_blocking(move || codec::process_message(&body, &dispatcher))
        .await
        .expect("dispatch task panicked");
    match result {
        Some(json) => {
            (StatusCode::OK, [("content-type", "application/json")], json).into_response()
        }
        None => StatusCode::NO_CONTENT.into_response(),
    }
}

/// POST with streaming tool → SSE response with event IDs.
fn handle_post_streaming(
    state: StreamableState,
    request: JsonRpcRequest,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = make_post_sse_stream(state, request);
    Sse::new(stream)
}

fn make_post_sse_stream(
    state: StreamableState,
    request: JsonRpcRequest,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let init = match state.dispatcher.dispatch_streaming(&request) {
        DispatchOutcome::Streaming {
            request_id,
            progress_rx,
            ctx,
            handler,
            arguments,
        } => {
            let id_str = request_id.to_string();
            state
                .active
                .lock()
                .unwrap()
                .insert(id_str.clone(), ctx.cancellation.clone());

            let tool_name = request
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let start = std::time::Instant::now();
            let handler_handle = tokio::task::spawn_blocking(move || handler(arguments, ctx));

            PostSseState::Running {
                progress_rx,
                handler_handle,
                request_id,
                id_str,
                active: state.active,
                dispatcher: state.dispatcher,
                event_ids: state.event_ids,
                resumption: state.resumption,
                tool_name,
                start,
                retry_ms: state.retry_ms,
            }
        }
        _ => PostSseState::Done,
    };

    futures_util::stream::unfold(init, |s| async move {
        match s {
            PostSseState::Running {
                progress_rx,
                handler_handle,
                request_id,
                id_str,
                active,
                dispatcher,
                event_ids,
                resumption,
                tool_name,
                start,
                retry_ms,
            } => {
                let recv_result = tokio::task::spawn_blocking(move || match progress_rx.recv() {
                    Ok(update) => PostRecvResult::Progress(update, progress_rx),
                    Err(_) => PostRecvResult::Done,
                })
                .await
                .expect("recv task panicked");

                match recv_result {
                    PostRecvResult::Progress(update, rx) => {
                        let evt_id = event_ids.next();
                        let notification =
                            crate::stream::progress_notification(&request_id, &update);
                        let data = serde_json::to_string(&notification).unwrap();
                        resumption.push(StreamEvent {
                            id: evt_id.clone(),
                            event: "progress".into(),
                            data: data.clone(),
                        });
                        let event = Event::default().event("progress").id(evt_id).data(data);
                        Some((
                            Ok(event),
                            PostSseState::Running {
                                progress_rx: rx,
                                handler_handle,
                                request_id,
                                id_str,
                                active,
                                dispatcher,
                                event_ids,
                                resumption,
                                tool_name,
                                start,
                                retry_ms,
                            },
                        ))
                    }
                    PostRecvResult::Done => {
                        let (response, success, error) = match handler_handle.await {
                            Ok(result) => {
                                (JsonRpcResponse::success(request_id, result), true, None)
                            }
                            Err(e) if e.is_cancelled() => (
                                JsonRpcResponse::error(request_id, -32800, "request cancelled"),
                                false,
                                Some("request cancelled".to_string()),
                            ),
                            Err(_) => (
                                JsonRpcResponse::error(
                                    request_id,
                                    -32603,
                                    "internal error: handler panicked",
                                ),
                                false,
                                Some("handler panicked".to_string()),
                            ),
                        };

                        let duration_ms = start.elapsed().as_millis() as u64;
                        dispatcher.log_tool_call(&crate::audit::ToolCallEvent {
                            tool_name,
                            duration_ms,
                            success,
                            error,
                            caller_id: None,
                        });

                        let evt_id = event_ids.next();
                        let data =
                            serde_json::to_string(&response).expect("BUG: response serialization");
                        resumption.push(StreamEvent {
                            id: evt_id.clone(),
                            event: "result".into(),
                            data: data.clone(),
                        });
                        let event = Event::default()
                            .event("result")
                            .id(evt_id)
                            .retry(Duration::from_millis(retry_ms))
                            .data(data);
                        active
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .remove(&id_str);
                        Some((Ok(event), PostSseState::Done))
                    }
                }
            }
            PostSseState::Done => None,
        }
    })
}

enum PostSseState {
    Running {
        progress_rx: std::sync::mpsc::Receiver<crate::stream::ProgressUpdate>,
        handler_handle: tokio::task::JoinHandle<serde_json::Value>,
        request_id: serde_json::Value,
        id_str: String,
        active: Arc<std::sync::Mutex<HashMap<String, CancellationToken>>>,
        dispatcher: Arc<Dispatcher>,
        event_ids: Arc<EventIdGenerator>,
        resumption: Arc<ResumptionBuffer>,
        tool_name: String,
        start: std::time::Instant,
        retry_ms: u64,
    },
    Done,
}

enum PostRecvResult {
    Progress(
        crate::stream::ProgressUpdate,
        std::sync::mpsc::Receiver<crate::stream::ProgressUpdate>,
    ),
    Done,
}

// ---------------------------------------------------------------------------
// GET handler — SSE stream for server-initiated messages
// ---------------------------------------------------------------------------

async fn handle_get(State(state): State<StreamableState>, headers: HeaderMap) -> Response {
    // --- Middleware ---
    if let Err(resp) = middleware::check_origin(&headers, &state.allowed_origins) {
        return resp;
    }
    if let Err(resp) = middleware::check_protocol_version_required(&headers) {
        return resp;
    }
    if let Err(resp) = middleware::check_session(&headers, &state.session_store, false) {
        return resp;
    }

    #[cfg(feature = "auth")]
    if let Err(resp) = middleware::check_bearer(
        &headers,
        &state.token_validator,
        &state.resource_metadata_url,
    ) {
        return resp;
    }

    // Check for resumption via Last-Event-ID.
    let last_event_id = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let stream = make_get_sse_stream(state, last_event_id);
    Sse::new(stream).into_response()
}

fn make_get_sse_stream(
    state: StreamableState,
    last_event_id: Option<String>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    // Replay missed events, then emit priming event.
    let mut replay_events: Vec<Event> = Vec::new();

    if let Some(ref last_id) = last_event_id {
        for missed in state.resumption.events_after(last_id) {
            let event = Event::default()
                .event(&missed.event)
                .id(missed.id)
                .data(missed.data);
            replay_events.push(event);
        }
    }

    // Priming event.
    let primer_id = state.event_ids.next();
    replay_events.push(
        Event::default()
            .event("message")
            .id(primer_id)
            .data(String::new()),
    );

    // Yield replay + primer, then the stream stays open for server-initiated messages.
    // For now, after replay we just keep the connection open (no server-initiated messages yet).
    let retry_ms = state.retry_ms;
    futures_util::stream::unfold(
        GetSseState::Replay(replay_events.into_iter(), retry_ms),
        |s| async move {
            match s {
                GetSseState::Replay(mut iter, retry_ms) => match iter.next() {
                    Some(event) => Some((Ok(event), GetSseState::Replay(iter, retry_ms))),
                    None => {
                        // Send retry hint before going idle.
                        let retry_event = Event::default()
                            .retry(Duration::from_millis(retry_ms))
                            .comment("stream open for server-initiated messages");
                        Some((Ok(retry_event), GetSseState::Open))
                    }
                },
                GetSseState::Open => {
                    // Keep connection open — future: server-initiated messages.
                    // For now, just hold indefinitely.
                    std::future::pending::<()>().await;
                    None
                }
            }
        },
    )
}

enum GetSseState {
    Replay(std::vec::IntoIter<Event>, u64),
    Open,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::Dispatcher;
    use crate::registry::{ToolDef, ToolRegistry, ToolSchema};
    use crate::session::MCP_PROTOCOL_VERSION_HEADER;
    use axum::body::Body;
    use axum::http::Request;
    use std::collections::HashMap;
    use tower::util::ServiceExt;

    fn make_streamable_app() -> Router {
        let mut reg = ToolRegistry::new();
        reg.register(ToolDef {
            name: "echo".into(),
            description: "Echo".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
            version: None,
            deprecated: None,
            annotations: None,
        });
        let mut d = Dispatcher::new(reg);
        d.handle(
            "echo",
            Arc::new(|params| {
                serde_json::json!({ "content": [{ "type": "text", "text": params.to_string() }] })
            }),
        );
        let config = StreamableConfig::new("127.0.0.1:0".parse().unwrap());
        let store = config
            .session_timeout
            .map(|t| Arc::new(SessionStore::new(t)));
        streamable_router(Arc::new(d), &config, store)
    }

    fn make_streamable_app_no_sessions() -> Router {
        let mut reg = ToolRegistry::new();
        reg.register(ToolDef {
            name: "echo".into(),
            description: "Echo".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
            version: None,
            deprecated: None,
            annotations: None,
        });
        let mut d = Dispatcher::new(reg);
        d.handle(
            "echo",
            Arc::new(|params| {
                serde_json::json!({ "content": [{ "type": "text", "text": params.to_string() }] })
            }),
        );
        let config = StreamableConfig::new("127.0.0.1:0".parse().unwrap()).without_sessions();
        streamable_router(Arc::new(d), &config, None)
    }

    // --- Type-level tests (existing) ---

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

    // --- Router tests ---

    #[tokio::test]
    async fn post_missing_protocol_version_returns_400() {
        let app = make_streamable_app_no_sessions();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn post_invalid_protocol_version_returns_400() {
        let app = make_streamable_app_no_sessions();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .header(MCP_PROTOCOL_VERSION_HEADER, "1999-01-01")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn post_initialize_returns_session_id() {
        let app = make_streamable_app();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .header(MCP_PROTOCOL_VERSION_HEADER, "2025-11-25")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get(MCP_SESSION_ID_HEADER).is_some());
    }

    #[tokio::test]
    async fn post_without_session_returns_404() {
        let app = make_streamable_app();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .header(MCP_PROTOCOL_VERSION_HEADER, "2025-11-25")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn post_initialize_no_sessions_still_works() {
        let app = make_streamable_app_no_sessions();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .header(MCP_PROTOCOL_VERSION_HEADER, "2025-11-25")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get(MCP_SESSION_ID_HEADER).is_none());
    }

    #[tokio::test]
    async fn post_tools_list_no_sessions() {
        let app = make_streamable_app_no_sessions();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .header(MCP_PROTOCOL_VERSION_HEADER, "2025-11-25")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_resp: JsonRpcResponse = serde_json::from_slice(&bytes).unwrap();
        let tools = rpc_resp.result.unwrap()["tools"].as_array().unwrap().len();
        assert_eq!(tools, 1);
    }

    #[tokio::test]
    async fn post_origin_rejected_returns_403() {
        let mut reg = ToolRegistry::new();
        reg.register(ToolDef {
            name: "echo".into(),
            description: "Echo".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
            version: None,
            deprecated: None,
            annotations: None,
        });
        let d = Dispatcher::new(reg);
        let config = StreamableConfig::new("127.0.0.1:0".parse().unwrap())
            .with_allowed_origins(vec!["http://localhost:3000".into()])
            .without_sessions();
        let app = streamable_router(Arc::new(d), &config, None);

        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .header("origin", "http://evil.com")
                    .header(MCP_PROTOCOL_VERSION_HEADER, "2025-11-25")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn graceful_shutdown_streamable() {
        let reg = ToolRegistry::new();
        let dispatcher = Arc::new(Dispatcher::new(reg));
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let config = StreamableConfig::new(addr);
        let handle = tokio::spawn(serve(dispatcher, config, async {
            rx.await.ok();
        }));

        tokio::time::sleep(Duration::from_millis(20)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
