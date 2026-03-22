//! Unix domain socket transport — newline-delimited JSON-RPC over a Unix socket.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::mpsc as tokio_mpsc;

use crate::dispatch::{DispatchOutcome, Dispatcher};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::stream::CancellationToken;
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
    let (reader, writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();

    // Outgoing message channel.
    let (out_tx, mut out_rx) = tokio_mpsc::unbounded_channel::<String>();

    // Active streaming requests for cancellation.
    let active: Arc<std::sync::Mutex<HashMap<String, CancellationToken>>> =
        Arc::new(std::sync::Mutex::new(HashMap::new()));

    // Writer task.
    let writer_handle = tokio::spawn(async move {
        let mut writer = writer;
        while let Some(msg) = out_rx.recv().await {
            if writer.write_all(format!("{msg}\n").as_bytes()).await.is_err() {
                break;
            }
        }
    });

    // Reader loop.
    while let Some(line) = lines.next_line().await? {
        if line.is_empty() {
            continue;
        }

        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
            // Check for cancellation.
            if req.method == "$/cancelRequest" {
                if let Some(target_id) = req.params.get("id").and_then(|v| v.as_str())
                    && let Some(token) = active.lock().unwrap_or_else(|e| e.into_inner()).get(target_id)
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

        // Non-streaming.
        let d = Arc::clone(&dispatcher);
        let tx = out_tx.clone();
        tokio::task::spawn_blocking(move || {
            if let Some(out) = codec::process_message(&line, &d)
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
            active.lock().unwrap_or_else(|e| e.into_inner()).insert(id_str.clone(), ctx.cancellation.clone());

            let handler_handle = tokio::task::spawn_blocking(move || handler(arguments, ctx));

            let progress_tx = out_tx.clone();
            let progress_req_id = req_id.clone();
            let progress_handle = tokio::task::spawn_blocking(move || {
                while let Ok(update) = progress_rx.recv() {
                    let notification = crate::stream::progress_notification(&progress_req_id, &update);
                    if let Ok(json) = serde_json::to_string(&notification) {
                        let _ = progress_tx.send(json);
                    }
                }
            });

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
            let _ = out_tx.send(serde_json::to_string(&response).expect("BUG: response serialization"));

            active.lock().unwrap_or_else(|e| e.into_inner()).remove(&id_str);
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
            description: "Slow".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
        });
        let mut d = Dispatcher::new(reg);
        d.handle("echo", Arc::new(|p| serde_json::json!({"echoed": p})));
        d.handle_streaming("slow", Arc::new(|_params, ctx| {
            for i in 1..=3 {
                if ctx.cancellation.is_cancelled() {
                    return serde_json::json!({"cancelled": true});
                }
                ctx.progress.report(i, 3);
                std::thread::sleep(Duration::from_millis(5));
            }
            serde_json::json!({"content": [{"type": "text", "text": "done"}]})
        }));
        Arc::new(d)
    }

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
        let (dir, sock_path) = test_sock_path("init2");
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
        let resp: JsonRpcResponse = serde_json::from_str(&resp_line).unwrap();
        assert!(resp.result.is_some());

        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn unix_multiple_requests() {
        let (dir, sock_path) = test_sock_path("multi2");
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

        let resp1: JsonRpcResponse =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        let resp2: JsonRpcResponse =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();

        // Order may vary with async dispatch — check both are present.
        let mut ids: Vec<serde_json::Value> = vec![resp1.id.clone(), resp2.id.clone()];
        ids.sort_by_key(|v| v.as_u64().unwrap_or(0));
        assert_eq!(ids, vec![serde_json::json!(1), serde_json::json!(2)]);

        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn unix_malformed_json() {
        let (dir, sock_path) = test_sock_path("bad2");
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
        let resp: JsonRpcResponse = serde_json::from_str(&resp_line).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32700);

        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn unix_graceful_shutdown() {
        let (dir, sock_path) = test_sock_path("shutdown2");
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let handle = tokio::spawn(serve(
            make_dispatcher(),
            UnixConfig { path: sock_path.clone() },
            async { rx.await.ok(); },
        ));

        let _stream = connect_retry(&sock_path).await;
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());

        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn unix_streaming_tool_progress_and_result() {
        let (dir, sock_path) = test_sock_path("stream");
        let (_tx, rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(serve(
            make_streaming_dispatcher(),
            UnixConfig { path: sock_path.clone() },
            async { rx.await.ok(); },
        ));

        let stream = connect_retry(&sock_path).await;
        let (reader, mut writer) = tokio::io::split(stream);
        let mut lines = BufReader::new(reader).lines();

        let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"slow","arguments":{}}}"#;
        writer.write_all(format!("{req}\n").as_bytes()).await.unwrap();

        let mut progress_count = 0;
        let mut final_result = None;

        for _ in 0..10 {
            let line = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
                .await
                .expect("timeout")
                .unwrap()
                .unwrap();
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();

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

        let _ = std::fs::remove_file(&sock_path);
        let _ = std::fs::remove_dir(&dir);
    }
}
