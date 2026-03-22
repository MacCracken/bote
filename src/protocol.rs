//! JSON-RPC 2.0 types for MCP.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request.
///
/// For normal requests, `id` is `Some(...)`. For notifications (no response
/// expected), `id` is `None` and the field is omitted during serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn new(id: impl Into<serde_json::Value>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(id.into()),
            method: method.into(),
            params: serde_json::Value::Null,
        }
    }

    /// Create a notification (a request with no `id` — the server must not reply).
    pub fn notification(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: None,
            method: method.into(),
            params: serde_json::Value::Null,
        }
    }

    /// Returns `true` if this is a notification (no `id` field).
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = params;
        self
    }
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: serde_json::Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_creation() {
        let req = JsonRpcRequest::new(1, "tools/list");
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, Some(serde_json::json!(1)));
        assert_eq!(req.method, "tools/list");
        assert!(!req.is_notification());
    }

    #[test]
    fn notification_creation() {
        let req = JsonRpcRequest::notification("notifications/initialized");
        assert_eq!(req.jsonrpc, "2.0");
        assert!(req.id.is_none());
        assert!(req.is_notification());
        assert_eq!(req.method, "notifications/initialized");
    }

    #[test]
    fn notification_serialization_omits_id() {
        let req = JsonRpcRequest::notification("notify");
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("\"id\""));
        assert!(json.contains("\"method\":\"notify\""));
    }

    #[test]
    fn notification_deserialization_without_id() {
        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert!(req.is_notification());
        assert!(req.id.is_none());
    }

    #[test]
    fn response_success() {
        let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"tools": []}));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_error() {
        let resp = JsonRpcResponse::error(serde_json::json!(1), -32601, "tool not found");
        assert!(resp.result.is_none());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[test]
    fn serde_roundtrip() {
        let req =
            JsonRpcRequest::new(42, "tools/call").with_params(serde_json::json!({"name": "test"}));
        let json = serde_json::to_string(&req).unwrap();
        let back: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.method, "tools/call");
        assert_eq!(back.id, Some(serde_json::json!(42)));
    }

    #[test]
    fn request_default_params_is_null() {
        let req = JsonRpcRequest::new(1, "initialize");
        assert!(req.params.is_null());
    }

    #[test]
    fn request_with_params_overrides() {
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "echo"}));
        assert_eq!(req.params["name"], "echo");
    }

    #[test]
    fn notification_with_params() {
        let req = JsonRpcRequest::notification("progress")
            .with_params(serde_json::json!({"percent": 50}));
        assert!(req.is_notification());
        assert_eq!(req.params["percent"], 50);
    }

    #[test]
    fn response_success_excludes_error() {
        let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!("ok"));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn response_error_excludes_result() {
        let resp = JsonRpcResponse::error(serde_json::json!(1), -32601, "not found");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn response_preserves_id() {
        let resp = JsonRpcResponse::success(serde_json::json!("abc-123"), serde_json::json!({}));
        assert_eq!(resp.id, serde_json::json!("abc-123"));
    }

    #[test]
    fn error_object_data_skipped_when_none() {
        let err = JsonRpcError { code: -32600, message: "bad".into(), data: None };
        let json = serde_json::to_string(&err).unwrap();
        assert!(!json.contains("\"data\""));
    }

    #[test]
    fn error_object_data_included_when_present() {
        let err = JsonRpcError {
            code: -32600,
            message: "bad".into(),
            data: Some(serde_json::json!({"detail": "more info"})),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"data\""));
        assert!(json.contains("more info"));
    }
}
