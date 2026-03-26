//! TypeScript bridge — SY-compatible HTTP endpoint with CORS and MCP result formatting.
//!
//! The bridge wraps bote's JSON-RPC dispatch in an HTTP server that:
//! - Adds CORS headers for cross-origin TypeScript clients
//! - Reformats `tools/call` results into SY's MCP envelope (`{ content: [{ type, text }] }`)
//! - Exposes a `/health` endpoint for liveness checks
//!
//! Enable the `bridge` feature to use this module.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing};

use crate::BoteError;
use crate::dispatch::Dispatcher;
use crate::protocol::JsonRpcResponse;
use crate::transport::codec;

/// Configuration for the TypeScript bridge server.
#[non_exhaustive]
pub struct BridgeConfig {
    /// Address to bind the bridge server.
    pub addr: SocketAddr,
    /// Allowed CORS origins. Use `["*"]` during development.
    pub allowed_origins: Vec<String>,
}

impl BridgeConfig {
    #[must_use]
    pub fn new(addr: SocketAddr, allowed_origins: Vec<String>) -> Self {
        Self {
            addr,
            allowed_origins,
        }
    }
}

/// Wrap a raw tool result into SY's MCP envelope format.
///
/// If the result already has a `content` array, it is returned as-is.
/// Otherwise the value is serialized to a text content block.
#[must_use]
#[inline]
pub fn wrap_tool_result(result: &serde_json::Value) -> serde_json::Value {
    // Already in MCP envelope shape — pass through.
    if result.get("content").and_then(|v| v.as_array()).is_some() {
        return result.clone();
    }

    // Wrap raw value into the expected envelope.
    serde_json::json!({
        "content": [{
            "type": "text",
            "text": result.to_string()
        }]
    })
}

/// Wrap an error result into SY's MCP envelope with the `isError` flag.
#[must_use]
#[inline]
fn wrap_error_result(message: &str) -> serde_json::Value {
    serde_json::json!({
        "content": [{
            "type": "text",
            "text": message
        }],
        "isError": true
    })
}

#[derive(Clone)]
struct BridgeState {
    dispatcher: Arc<Dispatcher>,
    allowed_origins: Arc<Vec<String>>,
}

/// Build the bridge axum router.
///
/// Composes JSON-RPC dispatch with CORS headers and MCP result formatting.
#[must_use = "build the axum router for the bridge"]
pub fn router(dispatcher: Arc<Dispatcher>, allowed_origins: Vec<String>) -> Router {
    let state = BridgeState {
        dispatcher,
        allowed_origins: Arc::new(allowed_origins),
    };
    Router::new()
        .route("/", routing::post(handle_rpc).options(handle_preflight))
        .route("/health", routing::get(handle_health))
        .with_state(state)
}

/// Start the bridge HTTP server.
///
/// Runs until the `shutdown` future resolves, then drains in-flight
/// requests and returns `Ok(())`.
pub async fn serve(
    dispatcher: Arc<Dispatcher>,
    config: BridgeConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> crate::Result<()> {
    let app = router(dispatcher, config.allowed_origins);

    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .map_err(|e| BoteError::BindFailed(e.to_string()))?;

    tracing::info!(addr = %config.addr, "bridge transport listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(BoteError::Io)?;

    tracing::info!("bridge transport shut down");
    Ok(())
}

async fn handle_rpc(State(state): State<BridgeState>, body: String) -> Response {
    let dispatcher = Arc::clone(&state.dispatcher);
    let result = tokio::task::spawn_blocking(move || process_bridge_message(&body, &dispatcher))
        .await
        .expect("BUG: dispatch task panicked");

    let mut response = match result {
        Some(json) => {
            (StatusCode::OK, [("content-type", "application/json")], json).into_response()
        }
        None => StatusCode::NO_CONTENT.into_response(),
    };

    apply_cors_headers(response.headers_mut(), &state.allowed_origins);
    response
}

async fn handle_preflight(State(state): State<BridgeState>) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    apply_cors_headers(response.headers_mut(), &state.allowed_origins);
    response
}

async fn handle_health(State(state): State<BridgeState>) -> Response {
    let mut response = (StatusCode::OK, "ok").into_response();
    apply_cors_headers(response.headers_mut(), &state.allowed_origins);
    response
}

fn apply_cors_headers(headers: &mut axum::http::HeaderMap, allowed_origins: &[String]) {
    let origin = if allowed_origins.iter().any(|o| o == "*") {
        HeaderValue::from_static("*")
    } else {
        // Join multiple origins — browsers only support one, but we list the first.
        // In production, callers should match the request Origin against allowed_origins.
        match allowed_origins.first() {
            Some(o) => HeaderValue::from_str(o).unwrap_or(HeaderValue::from_static("*")),
            None => HeaderValue::from_static("*"),
        }
    };

    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("POST, GET, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type"),
    );
}

/// Process a JSON-RPC message through the bridge, wrapping tool call results.
fn process_bridge_message(input: &str, dispatcher: &Dispatcher) -> Option<String> {
    // Parse as a single request to check if it's a tools/call.
    let value: serde_json::Value = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "bridge: JSON-RPC parse error");
            let resp = JsonRpcResponse::error(
                serde_json::json!(null),
                -32700,
                format!("parse error: {e}"),
            );
            return Some(serde_json::to_string(&resp).expect("BUG: response serialization"));
        }
    };

    // For non-object values (batch arrays, primitives), fall through to codec.
    if !value.is_object() {
        return codec::process_message(input, dispatcher);
    }

    let request: crate::protocol::JsonRpcRequest = match serde_json::from_value(value) {
        Ok(req) => req,
        Err(e) => {
            let resp = JsonRpcResponse::error(
                serde_json::json!(null),
                -32600,
                format!("invalid request: {e}"),
            );
            return Some(serde_json::to_string(&resp).expect("BUG: response serialization"));
        }
    };

    if request.jsonrpc != "2.0" {
        let resp = JsonRpcResponse::error(
            request.id.clone().unwrap_or(serde_json::Value::Null),
            -32600,
            format!(
                "invalid request: unsupported jsonrpc version '{}'",
                request.jsonrpc
            ),
        );
        return Some(serde_json::to_string(&resp).expect("BUG: response serialization"));
    }

    // Dispatch and wrap tools/call results.
    let resp = dispatcher.dispatch(&request)?;

    let wrapped = if request.method == "tools/call" {
        if let Some(result) = &resp.result {
            JsonRpcResponse::success(resp.id.clone(), wrap_tool_result(result))
        } else if let Some(err) = &resp.error {
            // Wrap error into MCP envelope.
            let mut wrapped_resp =
                JsonRpcResponse::success(resp.id.clone(), wrap_error_result(&err.message));
            // Keep the original error too for JSON-RPC compliance.
            wrapped_resp.error = resp.error.clone();
            wrapped_resp.result = Some(wrap_error_result(&err.message));
            wrapped_resp
        } else {
            resp
        }
    } else {
        resp
    };

    Some(serde_json::to_string(&wrapped).expect("BUG: response serialization"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::Dispatcher;
    use crate::registry::{ToolDef, ToolRegistry, ToolSchema};
    use axum::body::Body;
    use axum::http::Request;
    use std::collections::HashMap;
    use tower::util::ServiceExt;

    fn make_bridge_app() -> Router {
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
        });
        let mut d = Dispatcher::new(reg);
        d.handle(
            "echo",
            Arc::new(|params| serde_json::json!({"raw": params})),
        );
        // Also register a tool that returns MCP-formatted results.
        let mut reg2 = ToolRegistry::new();
        reg2.register(ToolDef {
            name: "echo".into(),
            description: "Echo".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
            version: None,
            deprecated: None,
        });
        reg2.register(ToolDef {
            name: "mcp_tool".into(),
            description: "MCP formatted".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
            version: None,
            deprecated: None,
        });
        let mut d = Dispatcher::new(reg2);
        d.handle(
            "echo",
            Arc::new(|params| serde_json::json!({"raw": params})),
        );
        d.handle(
            "mcp_tool",
            Arc::new(|_| {
                serde_json::json!({
                    "content": [{"type": "text", "text": "already formatted"}]
                })
            }),
        );
        router(Arc::new(d), vec!["*".into()])
    }

    // --- wrap_tool_result tests ---

    #[test]
    fn wrap_raw_value() {
        let raw = serde_json::json!({"answer": 42});
        let wrapped = wrap_tool_result(&raw);
        let content = wrapped["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert!(content[0]["text"].as_str().unwrap().contains("42"));
    }

    #[test]
    fn wrap_already_formatted() {
        let formatted = serde_json::json!({
            "content": [{"type": "text", "text": "hello"}]
        });
        let wrapped = wrap_tool_result(&formatted);
        assert_eq!(wrapped, formatted);
    }

    #[test]
    fn wrap_null_value() {
        let wrapped = wrap_tool_result(&serde_json::Value::Null);
        assert!(wrapped["content"].is_array());
        assert_eq!(wrapped["content"][0]["type"], "text");
    }

    #[test]
    fn wrap_string_value() {
        let wrapped = wrap_tool_result(&serde_json::json!("just a string"));
        let text = wrapped["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("just a string"));
    }

    #[test]
    fn wrap_error_has_is_error_flag() {
        let wrapped = wrap_error_result("something failed");
        assert_eq!(wrapped["isError"], true);
        assert_eq!(wrapped["content"][0]["text"], "something failed");
    }

    // --- Router tests ---

    #[tokio::test]
    async fn bridge_health() {
        let app = make_bridge_app();
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
        assert_eq!(
            resp.headers().get("access-control-allow-origin").unwrap(),
            "*"
        );
    }

    #[tokio::test]
    async fn bridge_cors_on_options() {
        let app = make_bridge_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert!(resp.headers().contains_key("access-control-allow-origin"));
        assert!(resp.headers().contains_key("access-control-allow-methods"));
        assert!(resp.headers().contains_key("access-control-allow-headers"));
    }

    #[tokio::test]
    async fn bridge_initialize() {
        let app = make_bridge_app();
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
        assert!(resp.headers().contains_key("access-control-allow-origin"));

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_resp: JsonRpcResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(rpc_resp.result.is_some());
    }

    #[tokio::test]
    async fn bridge_tool_call_wraps_result() {
        let app = make_bridge_app();
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "echo", "arguments": {"msg": "hi"}}
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
        let result = rpc_resp.result.unwrap();

        // Should be wrapped in MCP envelope.
        assert!(result["content"].is_array());
        assert_eq!(result["content"][0]["type"], "text");
    }

    #[tokio::test]
    async fn bridge_mcp_tool_passthrough() {
        let app = make_bridge_app();
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "mcp_tool", "arguments": {}}
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
        let result = rpc_resp.result.unwrap();

        // Already formatted — should pass through.
        assert_eq!(result["content"][0]["text"], "already formatted");
    }

    #[tokio::test]
    async fn bridge_notification_returns_204() {
        let app = make_bridge_app();
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
    async fn bridge_malformed_json() {
        let app = make_bridge_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from("not json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_resp: JsonRpcResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(rpc_resp.error.unwrap().code, -32700);
    }

    #[tokio::test]
    async fn bridge_graceful_shutdown() {
        let dispatcher = Arc::new(Dispatcher::new(ToolRegistry::new()));
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let handle = tokio::spawn(serve(
            dispatcher,
            BridgeConfig::new(addr, vec!["*".into()]),
            async {
                rx.await.ok();
            },
        ));

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
