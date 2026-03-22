//! Stdio transport — read/write JSON-RPC over stdin/stdout.

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};

/// Parse a JSON-RPC request from a line of input.
pub fn parse_request(line: &str) -> crate::Result<JsonRpcRequest> {
    Ok(serde_json::from_str(line)?)
}

/// Serialize a JSON-RPC response to a line of output.
pub fn serialize_response(response: &JsonRpcResponse) -> crate::Result<String> {
    Ok(serde_json::to_string(response)?)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let line = super::serialize_response(&resp).unwrap();
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
        let line = super::serialize_response(&resp).unwrap();
        assert!(line.contains("\"error\""));
        assert!(line.contains("-32601"));
        assert!(!line.contains("\"result\""));
    }

    #[test]
    fn parse_returns_json_error_variant() {
        let err = parse_request("{invalid").unwrap_err();
        assert_eq!(err.rpc_code(), -32700);
    }
}
