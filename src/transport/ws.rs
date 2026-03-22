//! WebSocket transport — bidirectional JSON-RPC over WebSocket.

use std::net::SocketAddr;
use std::sync::Arc;

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
/// Spawns a task per connection. Each connection reads text messages,
/// dispatches via the shared `Dispatcher`, and writes response messages back.
pub async fn serve(dispatcher: Arc<Dispatcher>, config: WsConfig) -> crate::Result<()> {
    let listener = TcpListener::bind(config.addr)
        .await
        .map_err(|e| crate::BoteError::BindFailed(e.to_string()))?;

    loop {
        let (stream, _) = listener.accept().await?;
        let dispatcher = Arc::clone(&dispatcher);

        tokio::spawn(async move {
            let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                Ok(ws) => ws,
                Err(_) => return,
            };

            use futures_util::{SinkExt, StreamExt};
            let (mut write, mut read) = ws_stream.split();

            while let Some(Ok(msg)) = read.next().await {
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
                    Err(e) => crate::protocol::JsonRpcResponse::error(
                        serde_json::json!(null),
                        e.rpc_code(),
                        e.to_string(),
                    ),
                };

                let out = match codec::serialize_response(&response) {
                    Ok(s) => s,
                    Err(_) => break,
                };

                if write.send(Message::Text(out.into())).await.is_err() {
                    break;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ToolDef, ToolRegistry, ToolSchema};
    use std::collections::HashMap;

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

    /// Start the WS server on an OS-assigned port, return the address.
    async fn start_server(dispatcher: Arc<Dispatcher>) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let dispatcher = Arc::clone(&dispatcher);
                tokio::spawn(async move {
                    let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
                    use futures_util::{SinkExt, StreamExt};
                    let (mut write, mut read) = ws_stream.split();

                    while let Some(Ok(msg)) = read.next().await {
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
                            Err(e) => crate::protocol::JsonRpcResponse::error(
                                serde_json::json!(null),
                                e.rpc_code(),
                                e.to_string(),
                            ),
                        };

                        let out = match codec::serialize_response(&response) {
                            Ok(s) => s,
                            Err(_) => break,
                        };

                        if write.send(Message::Text(out.into())).await.is_err() {
                            break;
                        }
                    }
                });
            }
        });

        addr
    }

    #[tokio::test]
    async fn ws_initialize() {
        let addr = start_server(make_dispatcher()).await;
        let url = format!("ws://{addr}");

        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        use futures_util::{SinkExt, StreamExt};
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
        let addr = start_server(make_dispatcher()).await;
        let url = format!("ws://{addr}");

        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        use futures_util::{SinkExt, StreamExt};
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
        let addr = start_server(make_dispatcher()).await;
        let url = format!("ws://{addr}");

        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        use futures_util::{SinkExt, StreamExt};

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
        let addr = start_server(make_dispatcher()).await;
        let url = format!("ws://{addr}");

        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        use futures_util::{SinkExt, StreamExt};
        ws.send(Message::Text("not json".into())).await.unwrap();

        let resp_msg = ws.next().await.unwrap().unwrap();
        let resp: crate::protocol::JsonRpcResponse =
            serde_json::from_str(&resp_msg.into_text().unwrap()).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32700);
    }
}
