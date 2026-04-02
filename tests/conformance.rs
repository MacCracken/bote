//! Protocol conformance test suite — systematic tests mapped to JSON-RPC 2.0 and MCP spec.
//!
//! Each test is self-contained and exercises the public API as an external consumer would.

use bote::dispatch::Dispatcher;
use bote::protocol::{JsonRpcRequest, JsonRpcResponse};
use bote::registry::{ToolDef, ToolRegistry, ToolSchema};
use bote::transport;
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_schema() -> ToolSchema {
    ToolSchema::new("object", HashMap::new(), vec![])
}

fn make_dispatcher() -> Dispatcher {
    let mut reg = ToolRegistry::new();
    reg.register(ToolDef::new("test_echo", "Echo", empty_schema()));
    reg.register(ToolDef::new("test_strict", "Strict", {
        let mut props = HashMap::new();
        props.insert("path".into(), serde_json::json!({"type": "string"}));
        ToolSchema::new("object", props, vec!["path".into()])
    }));
    let mut d = Dispatcher::new(reg);
    d.handle(
        "test_echo",
        Arc::new(|params| serde_json::json!({"echoed": params})),
    );
    d.handle("test_strict", Arc::new(|_| serde_json::json!({"ok": true})));
    d
}

fn parse_response(json: &str) -> JsonRpcResponse {
    serde_json::from_str(json).expect("failed to parse response JSON")
}

// ===========================================================================
// Section 4: Request Object
// ===========================================================================

#[test]
fn request_must_contain_jsonrpc_2_0() {
    let d = make_dispatcher();
    let input = r#"{"jsonrpc":"1.0","id":1,"method":"initialize"}"#;
    let out = transport::process_message(input, &d).unwrap();
    let resp = parse_response(&out);
    assert_eq!(resp.error.unwrap().code, -32600);
}

#[test]
fn request_missing_jsonrpc_field() {
    let d = make_dispatcher();
    let input = r#"{"id":1,"method":"initialize"}"#;
    let out = transport::process_message(input, &d).unwrap();
    let resp = parse_response(&out);
    assert_eq!(resp.error.unwrap().code, -32600);
}

#[test]
fn request_method_must_be_present() {
    let d = make_dispatcher();
    let input = r#"{"jsonrpc":"2.0","id":1}"#;
    let out = transport::process_message(input, &d).unwrap();
    let resp = parse_response(&out);
    assert!(resp.error.is_some());
}

#[test]
fn request_params_may_be_omitted() {
    let d = make_dispatcher();
    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
    let out = transport::process_message(input, &d).unwrap();
    let resp = parse_response(&out);
    assert!(resp.result.is_some());
}

// ===========================================================================
// Section 5: Response Object — ID Preservation
// ===========================================================================

#[test]
fn response_preserves_integer_id() {
    let d = make_dispatcher();
    let resp = d.dispatch(&JsonRpcRequest::new(42, "initialize")).unwrap();
    assert_eq!(resp.id, serde_json::json!(42));
}

#[test]
fn response_preserves_string_id() {
    let d = make_dispatcher();
    let resp = d
        .dispatch(&JsonRpcRequest::new("req-abc", "initialize"))
        .unwrap();
    assert_eq!(resp.id, serde_json::json!("req-abc"));
}

#[test]
fn response_preserves_null_id() {
    let d = make_dispatcher();
    let resp = d
        .dispatch(&JsonRpcRequest::new(serde_json::Value::Null, "initialize"))
        .unwrap();
    assert_eq!(resp.id, serde_json::Value::Null);
}

#[test]
fn success_response_has_result_no_error() {
    let d = make_dispatcher();
    let resp = d.dispatch(&JsonRpcRequest::new(1, "initialize")).unwrap();
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn error_response_has_error_no_result() {
    let d = make_dispatcher();
    let resp = d.dispatch(&JsonRpcRequest::new(1, "bogus/method")).unwrap();
    assert!(resp.result.is_none());
    assert!(resp.error.is_some());
}

// ===========================================================================
// Section 6: Notifications
// ===========================================================================

#[test]
fn notification_returns_none() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::notification("notifications/initialized");
    assert!(d.dispatch(&req).is_none());
}

#[test]
fn notification_via_process_message_returns_none() {
    let d = make_dispatcher();
    let input = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    assert!(transport::process_message(input, &d).is_none());
}

// ===========================================================================
// Section 7: Batch
// ===========================================================================

#[test]
fn batch_returns_array_of_responses() {
    let d = make_dispatcher();
    let input = r#"[
        {"jsonrpc":"2.0","id":1,"method":"initialize"},
        {"jsonrpc":"2.0","id":2,"method":"tools/list"}
    ]"#;
    let out = transport::process_message(input, &d).unwrap();
    let responses: Vec<JsonRpcResponse> = serde_json::from_str(&out).unwrap();
    assert_eq!(responses.len(), 2);
}

#[test]
fn empty_batch_returns_error() {
    let d = make_dispatcher();
    let out = transport::process_message("[]", &d).unwrap();
    let resp = parse_response(&out);
    assert_eq!(resp.error.unwrap().code, -32600);
}

#[test]
fn batch_mixed_with_notifications_omits_notification_responses() {
    let d = make_dispatcher();
    let input = r#"[
        {"jsonrpc":"2.0","id":1,"method":"initialize"},
        {"jsonrpc":"2.0","method":"notifications/initialized"},
        {"jsonrpc":"2.0","id":3,"method":"tools/list"}
    ]"#;
    let out = transport::process_message(input, &d).unwrap();
    let responses: Vec<JsonRpcResponse> = serde_json::from_str(&out).unwrap();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0].id, serde_json::json!(1));
    assert_eq!(responses[1].id, serde_json::json!(3));
}

#[test]
fn batch_all_notifications_returns_none() {
    let d = make_dispatcher();
    let input = r#"[
        {"jsonrpc":"2.0","method":"notify/a"},
        {"jsonrpc":"2.0","method":"notify/b"}
    ]"#;
    assert!(transport::process_message(input, &d).is_none());
}

#[test]
fn batch_with_invalid_element_returns_error_for_that_element() {
    let d = make_dispatcher();
    let input = r#"[
        {"jsonrpc":"2.0","id":1,"method":"initialize"},
        42,
        {"jsonrpc":"2.0","id":3,"method":"tools/list"}
    ]"#;
    let out = transport::process_message(input, &d).unwrap();
    let responses: Vec<JsonRpcResponse> = serde_json::from_str(&out).unwrap();
    assert_eq!(responses.len(), 3);
    assert!(responses[0].result.is_some());
    assert_eq!(responses[1].error.as_ref().unwrap().code, -32600);
    assert!(responses[2].result.is_some());
}

// ===========================================================================
// Error Codes
// ===========================================================================

#[test]
fn error_parse_error_code() {
    let d = make_dispatcher();
    let out = transport::process_message("not valid json", &d).unwrap();
    let resp = parse_response(&out);
    assert_eq!(resp.error.unwrap().code, -32700);
}

#[test]
fn error_invalid_request_non_object() {
    let d = make_dispatcher();
    for input in &["42", "\"hello\"", "true", "null"] {
        let out = transport::process_message(input, &d).unwrap();
        let resp = parse_response(&out);
        assert_eq!(resp.error.unwrap().code, -32600);
    }
}

#[test]
fn error_method_not_found() {
    let d = make_dispatcher();
    let resp = d
        .dispatch(&JsonRpcRequest::new(1, "nonexistent/method"))
        .unwrap();
    assert_eq!(resp.error.unwrap().code, -32601); // method not found per JSON-RPC 2.0 spec
}

#[test]
fn error_invalid_params_missing_name() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call").with_params(serde_json::json!({}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[test]
fn error_tool_not_found() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "nonexistent_tool", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32601);
}

#[test]
fn error_invalid_params_schema_violation() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "test_strict", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32602);
}

// ===========================================================================
// MCP: Initialize
// ===========================================================================

#[test]
fn initialize_returns_protocol_version() {
    let d = make_dispatcher();
    let resp = d.dispatch(&JsonRpcRequest::new(1, "initialize")).unwrap();
    let result = resp.result.unwrap();
    assert!(result["protocolVersion"].is_string());
}

#[test]
fn initialize_returns_capabilities() {
    let d = make_dispatcher();
    let resp = d.dispatch(&JsonRpcRequest::new(1, "initialize")).unwrap();
    let result = resp.result.unwrap();
    assert!(result["capabilities"]["tools"].is_object());
}

#[test]
fn initialize_returns_server_info() {
    let d = make_dispatcher();
    let resp = d.dispatch(&JsonRpcRequest::new(1, "initialize")).unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result["serverInfo"]["name"], "bote");
    assert!(result["serverInfo"]["version"].is_string());
}

#[test]
fn initialize_echoes_supported_version() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "initialize")
        .with_params(serde_json::json!({"protocolVersion": "2024-11-05"}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.result.unwrap()["protocolVersion"], "2024-11-05");
}

#[test]
fn initialize_falls_back_for_unsupported_version() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "initialize")
        .with_params(serde_json::json!({"protocolVersion": "2099-01-01"}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.result.unwrap()["protocolVersion"], "2025-03-26");
}

#[test]
fn initialize_uses_latest_when_version_missing() {
    let d = make_dispatcher();
    let resp = d.dispatch(&JsonRpcRequest::new(1, "initialize")).unwrap();
    assert_eq!(resp.result.unwrap()["protocolVersion"], "2025-03-26");
}

// ===========================================================================
// MCP: tools/list
// ===========================================================================

#[test]
fn tools_list_returns_registered_tools() {
    let d = make_dispatcher();
    let resp = d.dispatch(&JsonRpcRequest::new(1, "tools/list")).unwrap();
    let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
    assert_eq!(tools.len(), 2);
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"test_echo"));
    assert!(names.contains(&"test_strict"));
}

#[test]
fn tools_list_includes_schema() {
    let d = make_dispatcher();
    let resp = d.dispatch(&JsonRpcRequest::new(1, "tools/list")).unwrap();
    let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
    for tool in &tools {
        assert!(tool["inputSchema"].is_object());
        assert!(tool["description"].is_string());
    }
}

#[test]
fn tools_list_includes_version_when_present() {
    let mut reg = ToolRegistry::new();
    reg.register(ToolDef::new("v_tool", "Versioned", empty_schema()).with_version("1.0.0"));
    let d = Dispatcher::new(reg);
    let resp = d.dispatch(&JsonRpcRequest::new(1, "tools/list")).unwrap();
    let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
    assert_eq!(tools[0]["version"], "1.0.0");
}

#[test]
fn tools_list_includes_deprecated_when_present() {
    let mut reg = ToolRegistry::new();
    reg.register(ToolDef::new("old_tool", "Old", empty_schema()).with_deprecated("use new_tool"));
    let d = Dispatcher::new(reg);
    let resp = d.dispatch(&JsonRpcRequest::new(1, "tools/list")).unwrap();
    let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
    assert_eq!(tools[0]["deprecated"], "use new_tool");
}

// ===========================================================================
// MCP: tools/call
// ===========================================================================

#[test]
fn tools_call_success() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "test_echo", "arguments": {"msg": "hi"}}));
    let resp = d.dispatch(&req).unwrap();
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn tools_call_missing_name_returns_invalid_params() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call").with_params(serde_json::json!({}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[test]
fn tools_call_empty_name_returns_invalid_params() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[test]
fn tools_call_unknown_tool_returns_not_found() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "nonexistent", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32601);
}

#[test]
fn tools_call_invalid_params_returns_error() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "test_strict", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[test]
fn tools_call_valid_params_succeeds() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "test_strict", "arguments": {"path": "/tmp"}}));
    let resp = d.dispatch(&req).unwrap();
    assert!(resp.result.is_some());
}

#[test]
fn tools_call_unknown_version_returns_error() {
    let d = make_dispatcher();
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "test_echo", "version": "9.9.9", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[test]
fn tools_call_defaults_empty_arguments() {
    let d = make_dispatcher();
    let req =
        JsonRpcRequest::new(1, "tools/call").with_params(serde_json::json!({"name": "test_echo"}));
    let resp = d.dispatch(&req).unwrap();
    assert!(resp.result.is_some());
}

// ===========================================================================
// JSON-RPC 2.0 error code compliance
// ===========================================================================

#[test]
fn error_codes_comply_with_spec() {
    let d = make_dispatcher();

    // -32700: Parse error
    let out = transport::process_message("not json", &d).unwrap();
    let resp: JsonRpcResponse = serde_json::from_str(&out).unwrap();
    assert_eq!(resp.error.unwrap().code, -32700);

    // -32600: Invalid Request (wrong jsonrpc version)
    let out = transport::process_message(r#"{"jsonrpc":"1.0","id":1,"method":"initialize"}"#, &d)
        .unwrap();
    let resp: JsonRpcResponse = serde_json::from_str(&out).unwrap();
    assert_eq!(resp.error.unwrap().code, -32600);

    // -32601: Method not found
    let resp = d.dispatch(&JsonRpcRequest::new(1, "bogus/method")).unwrap();
    assert_eq!(resp.error.unwrap().code, -32601);

    // -32601: Tool not found (tools/call with nonexistent tool)
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "nope", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32601);

    // -32602: Invalid params
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "test_strict", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[cfg(feature = "bridge")]
#[test]
fn bridge_error_response_is_spec_compliant() {
    // Bridge error wrapping must not set both result AND error (JSON-RPC 2.0 violation).
    let mut reg = ToolRegistry::new();
    reg.register(ToolDef::new("test_missing", "Missing", {
        let mut props = HashMap::new();
        props.insert("path".into(), serde_json::json!({"type": "string"}));
        ToolSchema::new("object", props, vec!["path".into()])
    }));
    let mut d = Dispatcher::new(reg);
    d.handle(
        "test_missing",
        Arc::new(|_| serde_json::json!({"ok": true})),
    );
    let d = Arc::new(d);
    let app = bote::bridge::router(d, vec!["*".into()]);

    // Call with missing required param — triggers error path in bridge.
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "test_missing", "arguments": {}}
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use axum::body::Body;
        use axum::http::Request;
        use tower::util::ServiceExt;

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_resp: JsonRpcResponse = serde_json::from_slice(&bytes).unwrap();

        // Must have result (MCP envelope) but NOT also error.
        assert!(rpc_resp.result.is_some());
        assert!(
            rpc_resp.error.is_none(),
            "JSON-RPC 2.0 forbids setting both result and error"
        );
        // Result should have isError flag in the MCP envelope.
        let result = rpc_resp.result.unwrap();
        assert_eq!(result["isError"], true);
    });
}

// ===========================================================================
// Registry merge (audit: entries map covers both tool def and compiled schema)
// ===========================================================================

#[test]
fn registry_deregister_cleans_up_compiled_schema() {
    let mut reg = ToolRegistry::new();
    let mut props = HashMap::new();
    props.insert("path".into(), serde_json::json!({"type": "string"}));
    reg.register(ToolDef::new(
        "typed_tool",
        "Typed",
        ToolSchema::new("object", props, vec!["path".into()]),
    ));

    // Tool with compiled schema validates types.
    assert!(
        reg.validate_params("typed_tool", &serde_json::json!({"path": 42}))
            .is_err()
    );
    assert!(
        reg.validate_params("typed_tool", &serde_json::json!({"path": "/tmp"}))
            .is_ok()
    );

    // After deregister, tool no longer exists.
    reg.deregister("typed_tool");
    assert!(
        reg.validate_params("typed_tool", &serde_json::json!({"path": "/tmp"}))
            .is_err()
    );
    assert!(!reg.contains("typed_tool"));
}

// ===========================================================================
// Full flow: initialize → list → call
// ===========================================================================

#[test]
fn full_mcp_flow() {
    let d = make_dispatcher();

    // Initialize.
    let resp = d.dispatch(&JsonRpcRequest::new(1, "initialize")).unwrap();
    assert!(resp.result.is_some());

    // List tools.
    let resp = d.dispatch(&JsonRpcRequest::new(2, "tools/list")).unwrap();
    let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
    assert!(!tools.is_empty());

    // Call a tool (use test_echo which has no required params).
    let req = JsonRpcRequest::new(3, "tools/call")
        .with_params(serde_json::json!({"name": "test_echo", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert!(resp.result.is_some());
}
