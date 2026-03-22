//! Stdio transport — blocking line-oriented JSON-RPC over stdin/stdout.

use std::io::{BufRead, Write};

use crate::dispatch::Dispatcher;
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

        let request = match codec::parse_request(&line) {
            Ok(req) => req,
            Err(e) => {
                tracing::warn!(error = %e, "failed to parse JSON-RPC request");
                let err_resp = crate::protocol::JsonRpcResponse::error(
                    serde_json::json!(null),
                    e.rpc_code(),
                    e.to_string(),
                );
                let out = codec::serialize_response(&err_resp)?;
                writeln!(writer, "{out}")?;
                continue;
            }
        };

        let response = dispatcher.dispatch(&request);
        let out = codec::serialize_response(&response)?;
        writeln!(writer, "{out}")?;
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
}
