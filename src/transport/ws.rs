//! WebSocket transport — bidirectional JSON-RPC over WebSocket.

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc as tokio_mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::dispatch::{DispatchOutcome, Dispatcher};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::stream::CancellationToken;
use crate::transport::codec;

/// Configuration for the WebSocket transport.
pub struct WsConfig {
    pub addr: SocketAddr,
}

/// Start a WebSocket server that accepts JSON-RPC messages.
///
/// Spawns a task per connection. Runs until the `shutdown` future resolves,
/// then stops accepting new connections and returns `Ok(())`.
pub async fn serve(
    dispatcher: Arc<Dispatcher>,
    config: WsConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> crate::Result<()> {
    let listener = TcpListener::bind(config.addr)
        .await
        .map_err(|e| crate::BoteError::BindFailed(e.to_string()))?;

    tracing::info!(addr = %config.addr, "ws transport listening");

    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, peer) = result?;
                let dispatcher = Arc::clone(&dispatcher);

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(dispatcher, stream).await {
                        tracing::warn!(peer = %peer, error = %e, "ws connection error");
                    }
                });
            }
            _ = &mut shutdown => break,
        }
    }

    tracing::info!("ws transport shut down");
    Ok(())
}

async fn handle_connection(
    dispatcher: Arc<Dispatcher>,
    stream: tokio::net::TcpStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (ws_write, mut ws_read) = ws_stream.split();

    // Outgoing message channel — all tasks send here, writer drains to WS.
    let (out_tx, mut out_rx) = tokio_mpsc::unbounded_channel::<String>();

    // Active streaming requests for cancellation.
    let active: Arc<std::sync::Mutex<HashMap<String, CancellationToken>>> =
        Arc::new(std::sync::Mutex::new(HashMap::new()));

    // Writer task.
    let writer_handle = tokio::spawn(async move {
        let mut ws_write = ws_write;
        while let Some(msg) = out_rx.recv().await {
            if ws_write.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Reader loop.
    while let Some(msg) = ws_read.next().await {
        let msg = msg?;

        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };

        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&text) {
            // Check for cancellation request.
            if req.method == "$/cancelRequest" {
                if let Some(target_id) = req.params.get("id").and_then(|v| v.as_str())
                    && let Some(token) = active
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(target_id)
                {
                    token.cancel();
                }
                continue;
            }

            // Check for streaming tool.
            if req.method == "tools/call"
                && let Some(tool_name) = req.params.get("name").and_then(|v| v.as_str())
                && dispatcher.is_streaming_tool(tool_name)
            {
                let d = Arc::clone(&dispatcher);
                let tx = out_tx.clone();
                let active_map = Arc::clone(&active);

                tokio::spawn(async move {
                    handle_streaming_call(d, &req, tx, active_map).await;
                });
                continue;
            }
        }

        // Non-streaming: use process_message.
        let d = Arc::clone(&dispatcher);
        let tx = out_tx.clone();
        let text_owned = text.to_string();
        tokio::task::spawn_blocking(move || {
            if let Some(out) = codec::process_message(&text_owned, &d)
                && tx.send(out).is_err()
            {
                tracing::trace!("outbound channel closed, client disconnected");
            }
        });
    }

    drop(out_tx);
    let _ = writer_handle.await;
    Ok(())
}

async fn handle_streaming_call(
    dispatcher: Arc<Dispatcher>,
    request: &JsonRpcRequest,
    out_tx: tokio_mpsc::UnboundedSender<String>,
    active: Arc<std::sync::Mutex<HashMap<String, CancellationToken>>>,
) {
    let request_id = request.id.clone().unwrap_or(serde_json::Value::Null);
    let id_str = request_id.to_string();

    match dispatcher.dispatch_streaming(request) {
        DispatchOutcome::Streaming {
            request_id: req_id,
            progress_rx,
            ctx,
            handler,
            arguments,
        } => {
            // Track for cancellation.
            active
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(id_str.clone(), ctx.cancellation.clone());

            // Spawn handler on blocking thread.
            let handler_handle = tokio::task::spawn_blocking(move || handler(arguments, ctx));

            // Drain progress in background.
            let progress_tx = out_tx.clone();
            let progress_req_id = req_id.clone();
            let progress_handle = tokio::task::spawn_blocking(move || {
                while let Ok(update) = progress_rx.recv() {
                    let notification =
                        crate::stream::progress_notification(&progress_req_id, &update);
                    if let Ok(json) = serde_json::to_string(&notification) {
                        let _ = progress_tx.send(json);
                    }
                }
            });

            // Wait for both to complete.
            let _ = progress_handle.await;
            let response = match handler_handle.await {
                Ok(result) => JsonRpcResponse::success(req_id, result),
                Err(e) if e.is_cancelled() => {
                    tracing::info!("streaming handler cancelled");
                    JsonRpcResponse::error(req_id, -32800, "request cancelled")
                }
                Err(_) => {
                    tracing::error!("streaming handler panicked");
                    JsonRpcResponse::error(req_id, -32603, "internal error: handler panicked")
                }
            };
            let _ =
                out_tx.send(serde_json::to_string(&response).expect("BUG: response serialization"));

            // Remove from active.
            active
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&id_str);
        }
        DispatchOutcome::Immediate(Some(resp)) => {
            let _ = out_tx.send(serde_json::to_string(&resp).expect("BUG: response serialization"));
        }
        DispatchOutcome::Immediate(None) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ToolDef, ToolRegistry, ToolSchema};
    use std::collections::HashMap;
    use std::time::Duration;

    fn make_dispatcher() -> Arc<Dispatcher> {
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
        Arc::new(d)
    }

    fn make_streaming_dispatcher() -> Arc<Dispatcher> {
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
        reg.register(ToolDef {
            name: "slow".into(),
            description: "Slow streaming".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
        });
        let mut d = Dispatcher::new(reg);
        d.handle("echo", Arc::new(|p| serde_json::json!({"echoed": p})));
        d.handle_streaming(
            "slow",
            Arc::new(|_params, ctx| {
                for i in 1..=3 {
                    if ctx.cancellation.is_cancelled() {
                        return serde_json::json!({"cancelled": true});
                    }
                    ctx.progress.report(i, 3);
                    std::thread::sleep(Duration::from_millis(5));
                }
                serde_json::json!({"content": [{"type": "text", "text": "done"}]})
            }),
        );
        Arc::new(d)
    }

    async fn start_server(
        dispatcher: Arc<Dispatcher>,
    ) -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let config = WsConfig { addr };
        tokio::spawn(serve(dispatcher, config, async {
            rx.await.ok();
        }));

        for _ in 0..200 {
            if tokio::net::TcpStream::connect(addr).await.is_ok() {
                return (addr, tx);
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        panic!("ws server failed to start on {addr}");
    }

    #[tokio::test]
    async fn ws_initialize() {
        let (addr, _tx) = start_server(make_dispatcher()).await;
        let url = format!("ws://{addr}");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        ws.send(Message::Text(req.into())).await.unwrap();

        let resp_msg = ws.next().await.unwrap().unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&resp_msg.into_text().unwrap()).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.result.unwrap()["serverInfo"]["name"] == "bote");
    }

    #[tokio::test]
    async fn ws_tool_call() {
        let (addr, _tx) = start_server(make_dispatcher()).await;
        let url = format!("ws://{addr}");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{"msg":"hello"}}}"#;
        ws.send(Message::Text(req.into())).await.unwrap();

        let resp_msg = ws.next().await.unwrap().unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&resp_msg.into_text().unwrap()).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[tokio::test]
    async fn ws_multiple_messages() {
        let (addr, _tx) = start_server(make_dispatcher()).await;
        let url = format!("ws://{addr}");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        let req1 = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let req2 = r#"{"jsonrpc":"2.0","id":2,"method":"initialize"}"#;
        ws.send(Message::Text(req1.into())).await.unwrap();
        ws.send(Message::Text(req2.into())).await.unwrap();

        let resp1: JsonRpcResponse =
            serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap()).unwrap();
        let resp2: JsonRpcResponse =
            serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap()).unwrap();

        assert_eq!(resp1.id, serde_json::json!(1));
        assert_eq!(resp2.id, serde_json::json!(2));
        assert!(resp1.result.is_some());
        assert!(resp2.result.is_some());
    }

    #[tokio::test]
    async fn ws_malformed_json() {
        let (addr, _tx) = start_server(make_dispatcher()).await;
        let url = format!("ws://{addr}");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        ws.send(Message::Text("not json".into())).await.unwrap();

        let resp_msg = ws.next().await.unwrap().unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&resp_msg.into_text().unwrap()).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32700);
    }

    #[tokio::test]
    async fn ws_graceful_shutdown() {
        let (addr, tx) = start_server(make_dispatcher()).await;

        let url = format!("ws://{addr}");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        ws.send(Message::Text(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#.into(),
        ))
        .await
        .unwrap();
        let _ = ws.next().await.unwrap().unwrap();

        tx.send(()).unwrap();
    }

    #[tokio::test]
    async fn ws_streaming_tool_progress_and_result() {
        let (addr, _tx) = start_server(make_streaming_dispatcher()).await;
        let url = format!("ws://{addr}");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"slow","arguments":{}}}"#;
        ws.send(Message::Text(req.into())).await.unwrap();

        // Collect all messages until we get the final result.
        let mut progress_count = 0;
        let mut final_result = None;

        for _ in 0..10 {
            let msg = tokio::time::timeout(Duration::from_secs(2), ws.next())
                .await
                .expect("timeout waiting for message")
                .unwrap()
                .unwrap();
            let text = msg.into_text().unwrap();
            let v: serde_json::Value = serde_json::from_str(&text).unwrap();

            if v.get("method").and_then(|m| m.as_str()) == Some("notifications/progress") {
                progress_count += 1;
            } else if v.get("result").is_some() {
                final_result = Some(v);
                break;
            }
        }

        assert_eq!(progress_count, 3);
        let result = final_result.unwrap();
        assert_eq!(result["id"], 1);
        assert_eq!(result["result"]["content"][0]["text"], "done");
    }
}
