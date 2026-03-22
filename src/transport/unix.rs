//! Unix domain socket transport — newline-delimited JSON-RPC over a Unix socket.

use std::future::Future;
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
/// Spawns a task per connection. Runs until the `shutdown` future resolves,
/// then stops accepting new connections and returns `Ok(())`.
pub async fn serve(
    dispatcher: Arc<Dispatcher>,
    config: UnixConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> crate::Result<()> {
    // Remove stale socket file if it exists.
    let _ = std::fs::remove_file(&config.path);

    let listener = UnixListener::bind(&config.path)
        .map_err(|e| crate::BoteError::BindFailed(e.to_string()))?;

    tracing::info!(path = %config.path.display(), "unix transport listening");

    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                let dispatcher = Arc::clone(&dispatcher);

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(dispatcher, stream).await {
                        tracing::warn!(error = %e, "unix connection error");
                    }
                });
            }
            _ = &mut shutdown => break,
        }
    }

    tracing::info!("unix transport shut down");
    Ok(())
}

async fn handle_connection(
    dispatcher: Arc<Dispatcher>,
    stream: tokio::net::UnixStream,
) -> crate::Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
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
        writer.write_all(format!("{out}\n").as_bytes()).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ToolDef, ToolRegistry, ToolSchema};
    use std::collections::HashMap;
    use std::time::Duration;
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

    /// Connect to a Unix socket, retrying until the server is ready.
    async fn connect_retry(path: &std::path::Path) -> UnixStream {
        for _ in 0..200 {
            if let Ok(s) = UnixStream::connect(path).await {
                return s;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        panic!("failed to connect to unix socket at {}", path.display());
    }

    fn test_sock_path(name: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("bote-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.sock");
        (dir, path)
    }

    #[tokio::test]
    async fn unix_initialize() {
        let (dir, sock_path) = test_sock_path("init");
        let (_tx, rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(serve(
            make_dispatcher(),
            UnixConfig { path: sock_path.clone() },
            async { rx.await.ok(); },
        ));

        let stream = connect_retry(&sock_path).await;
        let (reader, mut writer) = tokio::io::split(stream);
        let mut lines = BufReader::new(reader).lines();

        let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        writer.write_all(format!("{req}\n").as_bytes()).await.unwrap();

        let resp_line = lines.next_line().await.unwrap().unwrap();
        let resp: crate::protocol::JsonRpcResponse = serde_json::from_str(&resp_line).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.result.unwrap()["serverInfo"]["name"] == "bote");

        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn unix_multiple_requests() {
        let (dir, sock_path) = test_sock_path("multi");
        let (_tx, rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(serve(
            make_dispatcher(),
            UnixConfig { path: sock_path.clone() },
            async { rx.await.ok(); },
        ));

        let stream = connect_retry(&sock_path).await;
        let (reader, mut writer) = tokio::io::split(stream);
        let mut lines = BufReader::new(reader).lines();

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
        let (dir, sock_path) = test_sock_path("bad");
        let (_tx, rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(serve(
            make_dispatcher(),
            UnixConfig { path: sock_path.clone() },
            async { rx.await.ok(); },
        ));

        let stream = connect_retry(&sock_path).await;
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

    #[tokio::test]
    async fn unix_graceful_shutdown() {
        let (dir, sock_path) = test_sock_path("shutdown");
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let handle = tokio::spawn(serve(
            make_dispatcher(),
            UnixConfig { path: sock_path.clone() },
            async { rx.await.ok(); },
        ));

        // Wait for bind, then verify it's up.
        let _stream = connect_retry(&sock_path).await;

        // Signal shutdown.
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());

        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }
}
