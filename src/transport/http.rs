//! HTTP transport — axum-based JSON-RPC server with SSE streaming.

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing};
use futures_util::stream::Stream;

use crate::BoteError;
use crate::dispatch::{DispatchOutcome, Dispatcher};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::stream::CancellationToken;
use crate::transport::codec;

/// Configuration for the HTTP transport.
#[non_exhaustive]
pub struct HttpConfig {
    pub addr: SocketAddr,
}

impl HttpConfig {
    #[must_use]
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

#[derive(Clone)]
struct AppState {
    dispatcher: Arc<Dispatcher>,
    active: Arc<std::sync::Mutex<HashMap<String, CancellationToken>>>,
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
    let app = router(dispatcher);

    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .map_err(|e| BoteError::BindFailed(e.to_string()))?;

    tracing::info!(addr = %config.addr, "http transport listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(BoteError::Io)?;

    tracing::info!("http transport shut down");
    Ok(())
}

/// Build the axum router. Exposed for testing without binding a port.
#[must_use = "build the axum router for the HTTP transport"]
pub fn router(dispatcher: Arc<Dispatcher>) -> Router {
    let state = AppState {
        dispatcher,
        active: Arc::new(std::sync::Mutex::new(HashMap::new())),
    };
    Router::new()
        .route("/", routing::post(handle_rpc))
        .route("/health", routing::get(handle_health))
        .with_state(state)
}

async fn handle_rpc(State(state): State<AppState>, body: String) -> Response {
    if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&body) {
        // Check for cancellation request.
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

        // Check for streaming tool.
        if req.method == "tools/call"
            && let Some(tool_name) = req.params.get("name").and_then(|v| v.as_str())
            && state.dispatcher.is_streaming_tool(tool_name)
        {
            return handle_streaming(state, req).into_response();
        }
    }

    // Non-streaming: use process_message.
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
    // Set up the streaming handler eagerly so we have a single unfold type.
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

            let handler_handle = tokio::task::spawn_blocking(move || handler(arguments, ctx));

            SseState::Running {
                progress_rx,
                handler_handle,
                request_id,
                id_str,
                active: state.active,
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
                            },
                        ))
                    }
                    RecvResult::Done => {
                        let response = match handler_handle.await {
                            Ok(result) => JsonRpcResponse::success(request_id, result),
                            Err(e) if e.is_cancelled() => {
                                tracing::info!("streaming handler cancelled");
                                JsonRpcResponse::error(request_id, -32800, "request cancelled")
                            }
                            Err(_) => {
                                tracing::error!("streaming handler panicked");
                                JsonRpcResponse::error(
                                    request_id,
                                    -32603,
                                    "internal error: handler panicked",
                                )
                            }
                        };
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
        assert_eq!(rpc_resp.error.unwrap().code, -32600);
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

        let handle = tokio::spawn(serve(dispatcher, HttpConfig { addr }, async {
            rx.await.ok();
        }));

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
