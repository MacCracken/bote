//! Stdio transport — blocking line-oriented JSON-RPC over stdin/stdout.

use std::io::{BufRead, Write};

use crate::dispatch::{DispatchOutcome, Dispatcher};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::transport::codec;

/// Run the stdio transport loop, reading JSON-RPC requests from stdin
/// and writing responses to stdout. Blocks until stdin is closed.
pub fn run(dispatcher: &Dispatcher) -> crate::Result<()> {
    let stdin = std::io::stdin().lock();
    let stdout = std::io::stdout().lock();
    run_io(dispatcher, stdin, stdout)
}

/// Internal: run the transport loop over arbitrary readers/writers (for testing).
fn run_io(
    dispatcher: &Dispatcher,
    reader: impl BufRead,
    mut writer: impl Write,
) -> crate::Result<()> {
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        // Try to parse as a single request to check for streaming.
        if let Ok(request) = codec::parse_request(&line)
            && let Some(tool_name) = extract_tool_name(&request)
            && dispatcher.is_streaming_tool(tool_name)
        {
            dispatch_streaming(dispatcher, &request, &mut writer)?;
            continue;
        }

        // Non-streaming: batch, notification, or sync tool.
        if let Some(out) = codec::process_message(&line, dispatcher) {
            writeln!(writer, "{out}")?;
        }
    }

    Ok(())
}

fn extract_tool_name(request: &JsonRpcRequest) -> Option<&str> {
    if request.method == "tools/call" {
        request.params.get("name").and_then(|v| v.as_str())
    } else {
        None
    }
}

fn dispatch_streaming(
    dispatcher: &Dispatcher,
    request: &JsonRpcRequest,
    writer: &mut impl Write,
) -> crate::Result<()> {
    match dispatcher.dispatch_streaming(request) {
        DispatchOutcome::Streaming {
            request_id: req_id,
            progress_rx,
            ctx,
            handler,
            arguments,
        } => {
            // Spawn handler on a thread.
            let handle = std::thread::spawn(move || handler(arguments, ctx));

            // Drain progress, writing notifications as JSON lines.
            while let Ok(update) = progress_rx.recv() {
                let notification = crate::stream::progress_notification(&req_id, &update);
                if let Ok(json) = serde_json::to_string(&notification) {
                    writeln!(writer, "{json}")?;
                }
            }

            // Write final result.
            let result = match handle.join() {
                Ok(v) => JsonRpcResponse::success(req_id, v),
                Err(_) => {
                    tracing::error!("streaming handler panicked");
                    JsonRpcResponse::error(req_id, -32603, "internal error: handler panicked")
                }
            };
            writeln!(writer, "{}", codec::serialize_response(&result)?)?;
        }
        DispatchOutcome::Immediate(Some(resp)) => {
            writeln!(writer, "{}", codec::serialize_response(&resp)?)?;
        }
        DispatchOutcome::Immediate(None) => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ToolDef, ToolRegistry, ToolSchema};
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::sync::Arc;

    fn make_dispatcher() -> Dispatcher {
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
        d
    }

    fn make_streaming_dispatcher() -> Dispatcher {
        let mut reg = ToolRegistry::new();
        reg.register(ToolDef {
            name: "slow".into(),
            description: "Slow".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
        });
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
        d.handle("echo", Arc::new(|params| serde_json::json!({"echoed": params})));
        d.handle_streaming("slow", Arc::new(|_params, ctx| {
            ctx.progress.report(1, 3);
            ctx.progress.report(2, 3);
            ctx.progress.report(3, 3);
            serde_json::json!({"content": [{"type": "text", "text": "done"}]})
        }));
        d
    }

    #[test]
    fn stdio_single_request() {
        let d = make_dispatcher();
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        let reader = Cursor::new(format!("{input}\n"));
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        assert!(out_str.contains("\"result\""));
        assert!(out_str.contains("bote"));
    }

    #[test]
    fn stdio_multiple_requests() {
        let d = make_dispatcher();
        let input = [
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"msg":"hi"}}}"#,
        ]
        .join("\n");
        let reader = Cursor::new(format!("{input}\n"));
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = out_str.lines().collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn stdio_skips_empty_lines() {
        let d = make_dispatcher();
        let input = "\n\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n\n";
        let reader = Cursor::new(input);
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        assert_eq!(out_str.lines().count(), 1);
    }

    #[test]
    fn stdio_malformed_json_returns_error() {
        let d = make_dispatcher();
        let input = "not valid json\n";
        let reader = Cursor::new(input);
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        assert!(out_str.contains("\"error\""));
        assert!(out_str.contains("-32700"));
    }

    #[test]
    fn stdio_empty_input_returns_ok() {
        let d = make_dispatcher();
        let reader = Cursor::new("");
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        assert!(out_str.is_empty());
    }

    #[test]
    fn stdio_notification_no_response() {
        let d = make_dispatcher();
        let input = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let reader = Cursor::new(format!("{input}\n"));
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        assert!(out_str.is_empty());
    }

    #[test]
    fn stdio_batch_request() {
        let d = make_dispatcher();
        let input = r#"[{"jsonrpc":"2.0","id":1,"method":"initialize"},{"jsonrpc":"2.0","id":2,"method":"tools/list"}]"#;
        let reader = Cursor::new(format!("{input}\n"));
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        let responses: Vec<crate::protocol::JsonRpcResponse> =
            serde_json::from_str(out_str.trim()).unwrap();
        assert_eq!(responses.len(), 2);
    }

    #[test]
    fn stdio_batch_all_notifications_no_response() {
        let d = make_dispatcher();
        let input = r#"[{"jsonrpc":"2.0","method":"notify1"},{"jsonrpc":"2.0","method":"notify2"}]"#;
        let reader = Cursor::new(format!("{input}\n"));
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        assert!(out_str.is_empty());
    }

    #[test]
    fn stdio_streaming_tool_emits_progress_and_result() {
        let d = make_streaming_dispatcher();
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"slow","arguments":{}}}"#;
        let reader = Cursor::new(format!("{input}\n"));
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = out_str.lines().collect();
        // 3 progress notifications + 1 final result = 4 lines.
        assert_eq!(lines.len(), 4);

        // First 3 are progress notifications.
        for line in &lines[..3] {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(v["method"], "notifications/progress");
        }

        // Last is the final result.
        let result: JsonRpcResponse = serde_json::from_str(lines[3]).unwrap();
        assert!(result.result.is_some());
        assert_eq!(result.id, serde_json::json!(1));
    }

    #[test]
    fn stdio_sync_tool_still_works_with_streaming_dispatcher() {
        let d = make_streaming_dispatcher();
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{"msg":"hi"}}}"#;
        let reader = Cursor::new(format!("{input}\n"));
        let mut output = Vec::new();

        run_io(&d, reader, &mut output).unwrap();

        let out_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = out_str.lines().collect();
        // Single response, no progress.
        assert_eq!(lines.len(), 1);
        let resp: JsonRpcResponse = serde_json::from_str(lines[0]).unwrap();
        assert!(resp.result.is_some());
    }
}
