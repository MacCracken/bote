//! Unix domain socket transport — newline-delimited JSON-RPC over a Unix socket.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

use crate::dispatch::Dispatcher;
use crate::transport::codec;

/// Configuration for the Unix socket transport.
pub struct UnixConfig {
    pub path: PathBuf,
}

/// Start a Unix domain socket server that accepts newline-delimited JSON-RPC.
///
/// Spawns a task per connection. Each connection reads lines, dispatches via
/// the shared `Dispatcher`, and writes response lines back.
pub async fn serve(dispatcher: Arc<Dispatcher>, config: UnixConfig) -> crate::Result<()> {
    // Remove stale socket file if it exists.
    let _ = std::fs::remove_file(&config.path);

    let listener = UnixListener::bind(&config.path)
        .map_err(|e| crate::BoteError::BindFailed(e.to_string()))?;

    loop {
        let (stream, _) = listener.accept().await?;
        let dispatcher = Arc::clone(&dispatcher);

        tokio::spawn(async move {
            let (reader, mut writer) = tokio::io::split(stream);
            let mut lines = BufReader::new(reader).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() {
                    continue;
                }

                let response = match codec::parse_request(&line) {
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

                if writer.write_all(format!("{out}\n").as_bytes()).await.is_err() {
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
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

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

    #[tokio::test]
    async fn unix_initialize() {
        let dir = std::env::temp_dir().join(format!("bote-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sock_path = dir.join("test.sock");

        let dispatcher = make_dispatcher();
        let config = UnixConfig { path: sock_path.clone() };

        // Start server in background.
        tokio::spawn(serve(dispatcher, config));

        // Give the listener a moment to bind.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let stream = UnixStream::connect(&sock_path).await.unwrap();
        let (reader, mut writer) = tokio::io::split(stream);
        let mut lines = BufReader::new(reader).lines();

        let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        writer.write_all(format!("{req}\n").as_bytes()).await.unwrap();

        let resp_line = lines.next_line().await.unwrap().unwrap();
        let resp: crate::protocol::JsonRpcResponse = serde_json::from_str(&resp_line).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.result.unwrap()["serverInfo"]["name"] == "bote");

        // Cleanup
        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn unix_multiple_requests() {
        let dir = std::env::temp_dir().join(format!("bote-test-multi-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sock_path = dir.join("test.sock");

        let dispatcher = make_dispatcher();
        let config = UnixConfig { path: sock_path.clone() };

        tokio::spawn(serve(dispatcher, config));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let stream = UnixStream::connect(&sock_path).await.unwrap();
        let (reader, mut writer) = tokio::io::split(stream);
        let mut lines = BufReader::new(reader).lines();

        // Send two requests on the same connection.
        let req1 = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let req2 = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"echo","arguments":{"msg":"hi"}}}"#;
        writer.write_all(format!("{req1}\n{req2}\n").as_bytes()).await.unwrap();

        let resp1: crate::protocol::JsonRpcResponse =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        let resp2: crate::protocol::JsonRpcResponse =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();

        assert_eq!(resp1.id, serde_json::json!(1));
        assert_eq!(resp2.id, serde_json::json!(2));
        assert!(resp1.result.is_some());
        assert!(resp2.result.is_some());

        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn unix_malformed_json() {
        let dir = std::env::temp_dir().join(format!("bote-test-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sock_path = dir.join("test.sock");

        let dispatcher = make_dispatcher();
        let config = UnixConfig { path: sock_path.clone() };

        tokio::spawn(serve(dispatcher, config));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let stream = UnixStream::connect(&sock_path).await.unwrap();
        let (reader, mut writer) = tokio::io::split(stream);
        let mut lines = BufReader::new(reader).lines();

        writer.write_all(b"not json\n").await.unwrap();

        let resp_line = lines.next_line().await.unwrap().unwrap();
        let resp: crate::protocol::JsonRpcResponse = serde_json::from_str(&resp_line).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32700);

        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }
}
