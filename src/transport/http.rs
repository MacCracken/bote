//! HTTP transport — axum-based JSON-RPC server with SSE streaming.
//!
//! Supports MCP 2025-11-25 transport middleware:
//! - Origin validation (DNS rebinding protection)
//! - `MCP-Protocol-Version` header enforcement
//! - `MCP-Session-Id` session lifecycle
//! - Bearer token extraction (feature `auth`)
//! - Periodic session pruning

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing};
use futures_util::stream::Stream;

use crate::BoteError;
use crate::dispatch::{DispatchOutcome, Dispatcher};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::session::{MCP_SESSION_ID_HEADER, SessionStore};
use crate::stream::CancellationToken;
use crate::transport::codec;
use crate::transport::middleware;

/// Configuration for the HTTP transport.
#[non_exhaustive]
pub struct HttpConfig {
    /// Listen address.
    pub addr: SocketAddr,
    /// Allowed `Origin` header values for DNS rebinding protection.
    /// `["*"]` allows any origin (development only). Empty = reject all.
    pub allowed_origins: Vec<String>,
    /// Session timeout. `None` disables session enforcement.
    pub session_timeout: Option<Duration>,
    /// Auth configuration (feature `auth`).
    #[cfg(feature = "auth")]
    pub token_validator: Option<Arc<dyn crate::auth::TokenValidator>>,
    /// Resource metadata URL for `WWW-Authenticate` header (feature `auth`).
    #[cfg(feature = "auth")]
    pub resource_metadata_url: Option<String>,
}

impl HttpConfig {
    /// Create a new config with permissive defaults (no session, wildcard origin).
    #[must_use]
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            allowed_origins: vec!["*".into()],
            session_timeout: None,
            #[cfg(feature = "auth")]
            token_validator: None,
            #[cfg(feature = "auth")]
            resource_metadata_url: None,
        }
    }

    /// Set allowed origins.
    #[must_use]
    pub fn with_allowed_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    /// Enable session enforcement with the given timeout.
    #[must_use]
    pub fn with_session_timeout(mut self, timeout: Duration) -> Self {
        self.session_timeout = Some(timeout);
        self
    }

    /// Set the token validator (feature `auth`).
    #[cfg(feature = "auth")]
    #[must_use]
    pub fn with_token_validator(
        mut self,
        validator: Arc<dyn crate::auth::TokenValidator>,
        metadata_url: impl Into<String>,
    ) -> Self {
        self.token_validator = Some(validator);
        self.resource_metadata_url = Some(metadata_url.into());
        self
    }
}

/// Default prune interval for session cleanup.
const SESSION_PRUNE_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Clone)]
struct AppState {
    dispatcher: Arc<Dispatcher>,
    active: Arc<std::sync::Mutex<HashMap<String, CancellationToken>>>,
    session_store: Option<Arc<SessionStore>>,
    allowed_origins: Vec<String>,
    #[cfg(feature = "auth")]
    token_validator: Option<Arc<dyn crate::auth::TokenValidator>>,
    #[cfg(feature = "auth")]
    resource_metadata_url: Option<String>,
}

/// Start an HTTP server that accepts JSON-RPC requests via `POST /`.
///
/// Runs until the `shutdown` future resolves, then drains in-flight
/// requests and returns `Ok(())`.
pub async fn serve(
    dispatcher: Arc<Dispatcher>,
    config: HttpConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> crate::Result<()> {
    let session_store = config
        .session_timeout
        .map(|t| Arc::new(SessionStore::new(t)));

    let app = router_with_config(dispatcher, &config, session_store.clone());

    // Spawn session prune task if sessions are enabled.
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

    tracing::info!(addr = %config.addr, "http transport listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(BoteError::Io)?;

    if let Some(handle) = prune_handle {
        handle.abort();
    }

    tracing::info!("http transport shut down");
    Ok(())
}

/// Build the axum router with permissive defaults. Exposed for testing.
#[must_use = "build the axum router for the HTTP transport"]
pub fn router(dispatcher: Arc<Dispatcher>) -> Router {
    let config = HttpConfig::new("0.0.0.0:0".parse().unwrap());
    router_with_config(dispatcher, &config, None)
}

/// Build the axum router with full middleware config.
#[must_use = "build the axum router for the HTTP transport"]
fn router_with_config(
    dispatcher: Arc<Dispatcher>,
    config: &HttpConfig,
    session_store: Option<Arc<SessionStore>>,
) -> Router {
    let state = AppState {
        dispatcher,
        active: Arc::new(std::sync::Mutex::new(HashMap::new())),
        session_store,
        allowed_origins: config.allowed_origins.clone(),
        #[cfg(feature = "auth")]
        token_validator: config.token_validator.clone(),
        #[cfg(feature = "auth")]
        resource_metadata_url: config.resource_metadata_url.clone(),
    };
    Router::new()
        .route("/", routing::post(handle_rpc))
        .route("/health", routing::get(handle_health))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_rpc(State(state): State<AppState>, headers: HeaderMap, body: String) -> Response {
    // --- Middleware checks ---
    if let Err(resp) = middleware::check_origin(&headers, &state.allowed_origins) {
        return resp;
    }
    if let Err(resp) = middleware::check_protocol_version(&headers) {
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
        // Cancellation request.
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

        // Streaming tool.
        if req.method == "tools/call"
            && let Some(tool_name) = req.params.get("name").and_then(|v| v.as_str())
            && state.dispatcher.is_streaming_tool(tool_name)
        {
            return handle_streaming(state, req).into_response();
        }

        // Initialize — create session and return header.
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

fn handle_streaming(
    state: AppState,
    request: JsonRpcRequest,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = make_sse_stream(state, request);
    Sse::new(stream)
}

fn make_sse_stream(
    state: AppState,
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

            SseState::Running {
                progress_rx,
                handler_handle,
                request_id,
                id_str,
                active: state.active,
                dispatcher: state.dispatcher,
                tool_name,
                start,
            }
        }
        _ => SseState::Done,
    };

    futures_util::stream::unfold(init, |s| async move {
        match s {
            SseState::Running {
                progress_rx,
                handler_handle,
                request_id,
                id_str,
                active,
                dispatcher,
                tool_name,
                start,
            } => {
                let recv_result = tokio::task::spawn_blocking(move || match progress_rx.recv() {
                    Ok(update) => RecvResult::Progress(update, progress_rx),
                    Err(_) => RecvResult::Done,
                })
                .await
                .expect("recv task panicked");

                match recv_result {
                    RecvResult::Progress(update, rx) => {
                        let notification =
                            crate::stream::progress_notification(&request_id, &update);
                        let event = Event::default()
                            .event("progress")
                            .data(serde_json::to_string(&notification).unwrap());
                        Some((
                            Ok(event),
                            SseState::Running {
                                progress_rx: rx,
                                handler_handle,
                                request_id,
                                id_str,
                                active,
                                dispatcher,
                                tool_name,
                                start,
                            },
                        ))
                    }
                    RecvResult::Done => {
                        let (response, success, error) = match handler_handle.await {
                            Ok(result) => {
                                (JsonRpcResponse::success(request_id, result), true, None)
                            }
                            Err(e) if e.is_cancelled() => {
                                tracing::info!("streaming handler cancelled");
                                (
                                    JsonRpcResponse::error(request_id, -32800, "request cancelled"),
                                    false,
                                    Some("request cancelled".to_string()),
                                )
                            }
                            Err(_) => {
                                tracing::error!("streaming handler panicked");
                                (
                                    JsonRpcResponse::error(
                                        request_id,
                                        -32603,
                                        "internal error: handler panicked",
                                    ),
                                    false,
                                    Some("handler panicked".to_string()),
                                )
                            }
                        };

                        let duration_ms = start.elapsed().as_millis() as u64;
                        dispatcher.log_tool_call(&crate::audit::ToolCallEvent {
                            tool_name,
                            duration_ms,
                            success,
                            error,
                            caller_id: None,
                        });

                        let event = Event::default().event("result").data(
                            serde_json::to_string(&response).expect("BUG: response serialization"),
                        );
                        active
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .remove(&id_str);
                        Some((Ok(event), SseState::Done))
                    }
                }
            }
            SseState::Done => None,
        }
    })
}

enum SseState {
    Running {
        progress_rx: std::sync::mpsc::Receiver<crate::stream::ProgressUpdate>,
        handler_handle: tokio::task::JoinHandle<serde_json::Value>,
        request_id: serde_json::Value,
        id_str: String,
        active: Arc<std::sync::Mutex<HashMap<String, CancellationToken>>>,
        dispatcher: Arc<Dispatcher>,
        tool_name: String,
        start: std::time::Instant,
    },
    Done,
}

enum RecvResult {
    Progress(
        crate::stream::ProgressUpdate,
        std::sync::mpsc::Receiver<crate::stream::ProgressUpdate>,
    ),
    Done,
}

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ToolDef, ToolRegistry, ToolSchema};
    use crate::session::MCP_PROTOCOL_VERSION_HEADER;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::collections::HashMap;
    use tower::util::ServiceExt;

    fn make_app() -> Router {
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
        router(Arc::new(d))
    }

    fn make_app_with_sessions() -> Router {
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
        let config = HttpConfig::new("0.0.0.0:0".parse().unwrap())
            .with_session_timeout(Duration::from_secs(3600));
        let store = config
            .session_timeout
            .map(|t| Arc::new(SessionStore::new(t)));
        router_with_config(Arc::new(d), &config, store)
    }

    fn make_app_with_strict_origins() -> Router {
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
        let config = HttpConfig::new("0.0.0.0:0".parse().unwrap())
            .with_allowed_origins(vec!["http://localhost:3000".into()]);
        router_with_config(Arc::new(d), &config, None)
    }

    #[tokio::test]
    async fn health_endpoint() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rpc_initialize() {
        let app = make_app();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
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
        assert!(rpc_resp.result.is_some());
        assert!(rpc_resp.error.is_none());
    }

    #[tokio::test]
    async fn rpc_tools_list() {
        let app = make_app();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_resp: JsonRpcResponse = serde_json::from_slice(&bytes).unwrap();
        let tools = rpc_resp.result.unwrap()["tools"].as_array().unwrap().len();
        assert_eq!(tools, 1);
    }

    #[tokio::test]
    async fn rpc_tool_call() {
        let app = make_app();
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "echo", "arguments": {"msg": "hello"}}
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_resp: JsonRpcResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(rpc_resp.result.is_some());
        assert!(rpc_resp.error.is_none());
    }

    #[tokio::test]
    async fn rpc_unknown_method() {
        let app = make_app();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 4, "method": "bogus"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_resp: JsonRpcResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(rpc_resp.error.is_some());
        assert_eq!(rpc_resp.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn rpc_malformed_json() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from("not valid json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_resp: JsonRpcResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(rpc_resp.error.is_some());
        assert_eq!(rpc_resp.error.unwrap().code, -32700);
    }

    #[tokio::test]
    async fn rpc_notification_returns_204() {
        let app = make_app();
        let body = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn rpc_batch() {
        let app = make_app();
        let body = r#"[
            {"jsonrpc":"2.0","id":1,"method":"initialize"},
            {"jsonrpc":"2.0","id":2,"method":"tools/list"}
        ]"#;
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let responses: Vec<JsonRpcResponse> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(responses.len(), 2);
    }

    #[tokio::test]
    async fn graceful_shutdown() {
        let dispatcher = {
            let reg = ToolRegistry::new();
            Arc::new(Dispatcher::new(reg))
        };
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let handle = tokio::spawn(serve(dispatcher, HttpConfig::new(addr), async {
            rx.await.ok();
        }));

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // --- Origin enforcement tests ---

    #[tokio::test]
    async fn origin_rejected_returns_403() {
        let app = make_app_with_strict_origins();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .header("origin", "http://evil.com")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn origin_allowed_passes() {
        let app = make_app_with_strict_origins();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .header("origin", "http://localhost:3000")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn origin_missing_passes() {
        let app = make_app_with_strict_origins();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- Protocol version enforcement tests ---

    #[tokio::test]
    async fn protocol_version_invalid_returns_400() {
        let app = make_app();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
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
    async fn protocol_version_valid_passes() {
        let app = make_app();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .header(MCP_PROTOCOL_VERSION_HEADER, "2025-11-25")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protocol_version_missing_passes() {
        let app = make_app();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- Session enforcement tests ---

    #[tokio::test]
    async fn session_initialize_returns_session_id_header() {
        let app = make_app_with_sessions();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get(MCP_SESSION_ID_HEADER).is_some());
    }

    #[tokio::test]
    async fn session_missing_header_returns_404() {
        let app = make_app_with_sessions();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn session_invalid_id_returns_404() {
        let app = make_app_with_sessions();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .header(MCP_SESSION_ID_HEADER, "nonexistent-session")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn session_disabled_no_enforcement() {
        let app = make_app();
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
