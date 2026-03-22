//! WebSocket transport — bidirectional JSON-RPC over WebSocket.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

use crate::dispatch::Dispatcher;
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
    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = msg?;

        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };

        let response = match codec::parse_request(&text) {
            Ok(request) => {
                let d = Arc::clone(&dispatcher);
                tokio::task::spawn_blocking(move || d.dispatch(&request))
                    .await
                    .expect("dispatch task panicked")
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to parse JSON-RPC request");
                crate::protocol::JsonRpcResponse::error(
                    serde_json::json!(null),
                    e.rpc_code(),
                    e.to_string(),
                )
            }
        };

        let out = codec::serialize_response(&response)?;
        write.send(Message::Text(out.into())).await?;
    }

    Ok(())
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

    /// Start the WS server on an OS-assigned port via `serve()`, return the address.
    /// Uses retry-connect instead of sleep for deterministic startup.
    async fn start_server(dispatcher: Arc<Dispatcher>) -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let config = WsConfig { addr };
        tokio::spawn(serve(dispatcher, config, async { rx.await.ok(); }));

        // Retry-connect to confirm the server is up.
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
        let resp_text = resp_msg.into_text().unwrap();
        let resp: crate::protocol::JsonRpcResponse = serde_json::from_str(&resp_text).unwrap();
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
        let resp: crate::protocol::JsonRpcResponse =
            serde_json::from_str(&resp_msg.into_text().unwrap()).unwrap();
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

        let resp1: crate::protocol::JsonRpcResponse =
            serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap()).unwrap();
        let resp2: crate::protocol::JsonRpcResponse =
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
        let resp: crate::protocol::JsonRpcResponse =
            serde_json::from_str(&resp_msg.into_text().unwrap()).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32700);
    }

    #[tokio::test]
    async fn ws_graceful_shutdown() {
        let (addr, tx) = start_server(make_dispatcher()).await;

        // Verify the server is accepting connections.
        let url = format!("ws://{addr}");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        ws.send(Message::Text(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#.into()))
            .await
            .unwrap();
        let _ = ws.next().await.unwrap().unwrap();

        // Signal shutdown.
        tx.send(()).unwrap();
    }
}
