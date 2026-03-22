//! HTTP transport — axum-based JSON-RPC server.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router, routing};

use crate::dispatch::Dispatcher;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::BoteError;

/// Configuration for the HTTP transport.
pub struct HttpConfig {
    pub addr: SocketAddr,
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
pub fn router(dispatcher: Arc<Dispatcher>) -> Router {
    Router::new()
        .route("/", routing::post(handle_rpc))
        .route("/health", routing::get(handle_health))
        .with_state(dispatcher)
}

async fn handle_rpc(
    State(dispatcher): State<Arc<Dispatcher>>,
    body: String,
) -> Json<JsonRpcResponse> {
    let request: JsonRpcRequest = match serde_json::from_str(&body) {
        Ok(req) => req,
        Err(e) => {
            let err = BoteError::Parse(e.to_string());
            tracing::warn!(error = %err, "failed to parse JSON-RPC request");
            return Json(JsonRpcResponse::error(
                serde_json::json!(null),
                err.rpc_code(),
                err.to_string(),
            ));
        }
    };

    let response = tokio::task::spawn_blocking(move || dispatcher.dispatch(&request))
        .await
        .expect("dispatch task panicked");
    Json(response)
}

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use crate::registry::{ToolDef, ToolRegistry, ToolSchema};
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

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let rpc_resp: JsonRpcResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(rpc_resp.error.is_some());
        assert_eq!(rpc_resp.error.unwrap().code, -32700);
    }

    #[tokio::test]
    async fn graceful_shutdown() {
        let dispatcher = {
            let reg = ToolRegistry::new();
            Arc::new(Dispatcher::new(reg))
        };
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let config = HttpConfig { addr: "127.0.0.1:0".parse().unwrap() };

        let listener = tokio::net::TcpListener::bind(config.addr).await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let handle = tokio::spawn(serve(
            dispatcher,
            HttpConfig { addr },
            async { rx.await.ok(); },
        ));

        // Give it a moment to bind, then signal shutdown.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
