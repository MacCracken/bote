//! JSON-RPC codec — parse requests, serialize responses, process messages.

use crate::dispatch::Dispatcher;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};

/// Serialize a value to JSON. Our protocol types are guaranteed to serialize
/// successfully, so this uses expect — a failure here indicates a bug.
fn to_json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("BUG: failed to serialize protocol type")
}

/// Parse a JSON-RPC request from a line of input.
pub fn parse_request(line: &str) -> crate::Result<JsonRpcRequest> {
    Ok(serde_json::from_str(line)?)
}

/// Serialize a JSON-RPC response to a line of output.
pub fn serialize_response(response: &JsonRpcResponse) -> crate::Result<String> {
    Ok(serde_json::to_string(response)?)
}

/// Process a raw JSON-RPC message (single request, batch, or notification).
///
/// Returns `Some(json_string)` for responses, or `None` if no response is
/// needed (e.g., all notifications). Handles batch arrays per the JSON-RPC 2.0
/// spec: returns an array of responses, omitting entries for notifications.
pub fn process_message(input: &str, dispatcher: &Dispatcher) -> Option<String> {
    let value: serde_json::Value = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "JSON-RPC parse error");
            let resp = JsonRpcResponse::error(
                serde_json::json!(null),
                -32700,
                format!("parse error: {e}"),
            );
            return Some(to_json(&resp));
        }
    };

    match value {
        serde_json::Value::Array(items) => process_batch(items, dispatcher),
        serde_json::Value::Object(_) => process_single(value, dispatcher),
        _ => {
            tracing::warn!("invalid request: not an object or array");
            let resp = JsonRpcResponse::error(
                serde_json::json!(null),
                -32600,
                "invalid request: expected object or array",
            );
            Some(to_json(&resp))
        }
    }
}

fn process_single(value: serde_json::Value, dispatcher: &Dispatcher) -> Option<String> {
    let request: JsonRpcRequest = match serde_json::from_value(value) {
        Ok(req) => req,
        Err(e) => {
            let resp = JsonRpcResponse::error(
                serde_json::json!(null),
                -32600,
                format!("invalid request: {e}"),
            );
            return Some(to_json(&resp));
        }
    };

    if request.jsonrpc != "2.0" {
        tracing::warn!(version = %request.jsonrpc, "unsupported jsonrpc version");
        let resp = JsonRpcResponse::error(
            request.id.clone().unwrap_or(serde_json::Value::Null),
            -32600,
            format!("invalid request: unsupported jsonrpc version '{}'", request.jsonrpc),
        );
        return Some(to_json(&resp));
    }

    dispatcher
        .dispatch(&request)
        .map(|resp| to_json(&resp))
}

fn process_batch(items: Vec<serde_json::Value>, dispatcher: &Dispatcher) -> Option<String> {
    if items.is_empty() {
        let resp = JsonRpcResponse::error(
            serde_json::json!(null),
            -32600,
            "invalid request: empty batch",
        );
        return Some(to_json(&resp));
    }

    let responses: Vec<JsonRpcResponse> = items
        .into_iter()
        .filter_map(|item| {
            if !item.is_object() {
                let resp = JsonRpcResponse::error(
                    serde_json::json!(null),
                    -32600,
                    "invalid request: batch element is not an object",
                );
                return Some(resp);
            }

            let request: JsonRpcRequest = match serde_json::from_value(item) {
                Ok(req) => req,
                Err(e) => {
                    return Some(JsonRpcResponse::error(
                        serde_json::json!(null),
                        -32600,
                        format!("invalid request: {e}"),
                    ));
                }
            };

            dispatcher.dispatch(&request)
        })
        .collect();

    if responses.is_empty() {
        None
    } else {
        Some(to_json(&responses))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ToolDef, ToolRegistry, ToolSchema};
    use std::collections::HashMap;
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
            Arc::new(|params| serde_json::json!({"echoed": params})),
        );
        d
    }

    // --- Existing parse/serialize tests ---

    #[test]
    fn parse_valid_request() {
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        let req = parse_request(line).unwrap();
        assert_eq!(req.method, "tools/list");
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_request("not json").is_err());
    }

    #[test]
    fn serialize_roundtrip() {
        let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"ok": true}));
        let line = serialize_response(&resp).unwrap();
        assert!(line.contains("\"result\""));
        assert!(!line.contains("\"error\""));
    }

    #[test]
    fn parse_empty_string() {
        assert!(parse_request("").is_err());
    }

    #[test]
    fn parse_preserves_params() {
        let line = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"echo","arguments":{"msg":"hi"}}}"#;
        let req = parse_request(line).unwrap();
        assert_eq!(req.params["name"], "echo");
        assert_eq!(req.params["arguments"]["msg"], "hi");
    }

    #[test]
    fn serialize_error_response() {
        let resp = JsonRpcResponse::error(serde_json::json!(1), -32601, "not found");
        let line = serialize_response(&resp).unwrap();
        assert!(line.contains("\"error\""));
        assert!(line.contains("-32601"));
        assert!(!line.contains("\"result\""));
    }

    #[test]
    fn parse_returns_json_error_variant() {
        let err = parse_request("{invalid").unwrap_err();
        assert_eq!(err.rpc_code(), -32700);
    }

    #[test]
    fn parse_notification() {
        let line = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let req = parse_request(line).unwrap();
        assert!(req.is_notification());
    }

    // --- process_message tests ---

    #[test]
    fn process_single_request() {
        let d = make_dispatcher();
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        let out = process_message(input, &d).unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&out).unwrap();
        assert!(resp.result.is_some());
        assert_eq!(resp.id, serde_json::json!(1));
    }

    #[test]
    fn process_notification_returns_none() {
        let d = make_dispatcher();
        let input = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        assert!(process_message(input, &d).is_none());
    }

    #[test]
    fn process_batch() {
        let d = make_dispatcher();
        let input = r#"[
            {"jsonrpc":"2.0","id":1,"method":"initialize"},
            {"jsonrpc":"2.0","id":2,"method":"tools/list"}
        ]"#;
        let out = process_message(input, &d).unwrap();
        let responses: Vec<JsonRpcResponse> = serde_json::from_str(&out).unwrap();
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].id, serde_json::json!(1));
        assert_eq!(responses[1].id, serde_json::json!(2));
    }

    #[test]
    fn process_batch_mixed_with_notifications() {
        let d = make_dispatcher();
        let input = r#"[
            {"jsonrpc":"2.0","id":1,"method":"initialize"},
            {"jsonrpc":"2.0","method":"notifications/initialized"},
            {"jsonrpc":"2.0","id":3,"method":"tools/list"}
        ]"#;
        let out = process_message(input, &d).unwrap();
        let responses: Vec<JsonRpcResponse> = serde_json::from_str(&out).unwrap();
        // Notification produces no response entry.
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].id, serde_json::json!(1));
        assert_eq!(responses[1].id, serde_json::json!(3));
    }

    #[test]
    fn process_batch_all_notifications_returns_none() {
        let d = make_dispatcher();
        let input = r#"[
            {"jsonrpc":"2.0","method":"notifications/initialized"},
            {"jsonrpc":"2.0","method":"notifications/progress"}
        ]"#;
        assert!(process_message(input, &d).is_none());
    }

    #[test]
    fn process_empty_batch_returns_error() {
        let d = make_dispatcher();
        let out = process_message("[]", &d).unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&out).unwrap();
        assert_eq!(resp.error.unwrap().code, -32600);
    }

    #[test]
    fn process_non_json_returns_error() {
        let d = make_dispatcher();
        let out = process_message("not json at all", &d).unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&out).unwrap();
        assert_eq!(resp.error.unwrap().code, -32700);
    }

    #[test]
    fn process_non_object_non_array_returns_error() {
        let d = make_dispatcher();
        for input in &["42", "\"hello\"", "true", "null"] {
            let out = process_message(input, &d).unwrap();
            let resp: JsonRpcResponse = serde_json::from_str(&out).unwrap();
            assert_eq!(resp.error.unwrap().code, -32600);
        }
    }

    #[test]
    fn process_wrong_jsonrpc_version() {
        let d = make_dispatcher();
        let input = r#"{"jsonrpc":"1.0","id":1,"method":"initialize"}"#;
        let out = process_message(input, &d).unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&out).unwrap();
        assert_eq!(resp.error.as_ref().unwrap().code, -32600);
        assert!(resp.error.unwrap().message.contains("unsupported jsonrpc version"));
    }

    #[test]
    fn process_missing_jsonrpc_field() {
        let d = make_dispatcher();
        // Missing jsonrpc field — fails deserialization
        let input = r#"{"id":1,"method":"initialize"}"#;
        let out = process_message(input, &d).unwrap();
        let resp: JsonRpcResponse = serde_json::from_str(&out).unwrap();
        assert_eq!(resp.error.as_ref().unwrap().code, -32600);
    }

    #[test]
    fn process_batch_with_invalid_element() {
        let d = make_dispatcher();
        let input = r#"[
            {"jsonrpc":"2.0","id":1,"method":"initialize"},
            42,
            {"jsonrpc":"2.0","id":3,"method":"tools/list"}
        ]"#;
        let out = process_message(input, &d).unwrap();
        let responses: Vec<JsonRpcResponse> = serde_json::from_str(&out).unwrap();
        assert_eq!(responses.len(), 3);
        // First and third are successful.
        assert!(responses[0].result.is_some());
        assert!(responses[2].result.is_some());
        // Second is an error for the invalid element.
        assert_eq!(responses[1].error.as_ref().unwrap().code, -32600);
        assert_eq!(responses[1].id, serde_json::json!(null));
    }
}
